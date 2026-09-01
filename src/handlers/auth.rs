//! OAuth 2.0 authorization-code login for browsers. API clients (Binary
//! Ninja) authenticate with API keys instead; see [`crate::auth`].

use actix_web::{HttpRequest, HttpResponse, get, http::header, post, web};
use chrono::{Duration, Utc};
use serde_json::Value;
use url::Url;

use crate::{
    auth::{self, SESSION_COOKIE, generate_secret},
    config::{Config, OAuthConfig},
    db::{Db, now, users},
    error::{ApiError, ApiResult},
    models::{auth::AuthUrlResponse, core::Role, pagination::NextUrl, request::OAuthCallback},
};

const STATE_TTL_MINUTES: i64 = 15;

fn oauth(config: &Config) -> ApiResult<&OAuthConfig> {
    config.oauth.as_ref().ok_or_else(|| {
        ApiError::ServiceUnavailable(
            "OAuth login is not configured on this server; authenticate with an API key".into(),
        )
    })
}

/// Start a login: returns the provider URL to send the browser to.
#[post("/login")]
pub async fn login(
    pool: web::Data<Db>,
    config: web::Data<Config>,
    body: Option<web::Json<NextUrl>>,
) -> ApiResult<HttpResponse> {
    let oauth = oauth(&config)?;
    let next = body.and_then(|b| b.next.clone());
    let state = generate_secret();

    sqlx::query("DELETE FROM oauth_states WHERE created_at < ?")
        .bind((Utc::now() - Duration::minutes(STATE_TTL_MINUTES)).to_rfc3339())
        .execute(pool.get_ref())
        .await?;
    sqlx::query("INSERT INTO oauth_states (state, next, created_at) VALUES (?, ?, ?)")
        .bind(&state)
        .bind(next)
        .bind(now())
        .execute(pool.get_ref())
        .await?;

    let auth_url = Url::parse_with_params(
        &oauth.auth_url,
        &[
            ("response_type", "code"),
            ("client_id", oauth.client_id.as_str()),
            ("redirect_uri", oauth.redirect_url.as_str()),
            ("scope", oauth.scopes.as_str()),
            ("state", state.as_str()),
        ],
    )
    .map_err(|e| ApiError::Internal(format!("invalid OAUTH_AUTH_URL: {e}")))?;

    Ok(HttpResponse::Ok().json(AuthUrlResponse {
        auth_url: auth_url.into(),
    }))
}

#[post("/logout")]
pub async fn logout(pool: web::Data<Db>, req: HttpRequest) -> ApiResult<HttpResponse> {
    if let Some(cookie) = req.cookie(SESSION_COOKIE) {
        auth::delete_session(pool.get_ref(), cookie.value()).await?;
    }
    Ok(HttpResponse::NoContent()
        .cookie(auth::clear_session_cookie())
        .finish())
}

/// Only allow same-site redirects after login.
fn safe_redirect(next: Option<String>, public_url: &str) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") => n,
        Some(n) if n.starts_with(public_url) => n,
        _ => "/".to_string(),
    }
}

#[get("/o/callback")]
pub async fn get_callback(
    pool: web::Data<Db>,
    config: web::Data<Config>,
    query: web::Query<OAuthCallback>,
) -> ApiResult<HttpResponse> {
    let oauth = oauth(&config)?;

    let state: Option<(Option<String>, String)> =
        sqlx::query_as("SELECT next, created_at FROM oauth_states WHERE state = ?")
            .bind(&query.state)
            .fetch_optional(pool.get_ref())
            .await?;
    let Some((next, created_at)) = state else {
        return Err(ApiError::bad_request("unknown or expired OAuth state"));
    };
    sqlx::query("DELETE FROM oauth_states WHERE state = ?")
        .bind(&query.state)
        .execute(pool.get_ref())
        .await?;
    let fresh = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map(|t| Utc::now() - t.with_timezone(&Utc) < Duration::minutes(STATE_TTL_MINUTES))
        .unwrap_or(false);
    if !fresh {
        return Err(ApiError::bad_request(
            "OAuth state has expired; start the login again",
        ));
    }

    let client = reqwest::Client::new();
    let token_response = client
        .post(&oauth.token_url)
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", query.code.as_str()),
            ("redirect_uri", oauth.redirect_url.as_str()),
            ("client_id", oauth.client_id.as_str()),
            ("client_secret", oauth.client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("OAuth token request failed: {e}")))?;
    if !token_response.status().is_success() {
        tracing::warn!(status = %token_response.status(), "OAuth token exchange rejected");
        return Err(ApiError::Unauthorized);
    }
    let token: Value = token_response
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("OAuth token response was not JSON: {e}")))?;
    let access_token = token
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or(ApiError::Unauthorized)?;

    let info_response = client
        .get(&oauth.userinfo_url)
        .header("Accept", "application/json")
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("OAuth user-info request failed: {e}")))?;
    if !info_response.status().is_success() {
        return Err(ApiError::Unauthorized);
    }
    let info: Value = info_response
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("OAuth user-info response was not JSON: {e}")))?;
    let email = info
        .get(&oauth.email_field)
        .and_then(Value::as_str)
        .filter(|e| !e.is_empty())
        .ok_or_else(|| {
            tracing::warn!(field = %oauth.email_field, "OAuth user-info has no e-mail");
            ApiError::Unauthorized
        })?;

    let mut conn = pool.acquire().await?;
    let user = match users::find_by_email(&mut *conn, email).await? {
        Some(user) => user,
        None => {
            if !config.auto_register {
                return Err(ApiError::Unauthorized);
            }
            let username = info
                .get(&oauth.username_field)
                .or_else(|| info.get("name"))
                .and_then(Value::as_str);
            // The very first account on a fresh server becomes the admin.
            let role = if users::count(&mut *conn).await? == 0 {
                Role::Admin
            } else {
                Role::User
            };
            users::create(
                &mut conn,
                email,
                username,
                role,
                users::UsernamePolicy::Deduplicate,
            )
            .await?
        }
    };
    drop(conn);

    let ttl = config.session_ttl();
    let token = auth::create_session(pool.get_ref(), user.id, ttl).await?;
    tracing::info!(user = %user.username, "user logged in via OAuth");

    Ok(HttpResponse::Found()
        .insert_header((header::LOCATION, safe_redirect(next, &config.public_url)))
        .cookie(auth::session_cookie(&token, ttl))
        .finish())
}
