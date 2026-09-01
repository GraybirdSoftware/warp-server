//! Authentication: API keys (`Authorization: Bearer warp_...`) and browser
//! sessions created by the OAuth flow.

use std::{future::Future, pin::Pin};

use actix_web::{
    FromRequest, HttpRequest,
    cookie::{Cookie, SameSite},
    dev::Payload,
    http::header::AUTHORIZATION,
    web,
};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Executor, FromRow, Sqlite};

use crate::{
    db::{Db, now},
    error::{ApiError, ApiResult},
    models::core::Role,
};

pub const SESSION_COOKIE: &str = "warp_session";
pub const API_KEY_PREFIX: &str = "warp_";

/// Generate a new API key. Only the SHA-256 digest is ever stored.
pub fn generate_token() -> String {
    let bytes: [u8; 24] = rand::random();
    format!("{API_KEY_PREFIX}{}", hex::encode(bytes))
}

/// Random opaque secret (sessions, OAuth state).
pub fn generate_secret() -> String {
    hex::encode(rand::random::<[u8; 24]>())
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// The authenticated caller. Extract it as a handler argument to require auth.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: Role,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    pub fn require_admin(&self) -> ApiResult<()> {
        if self.is_admin() {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }

    pub fn require_self_or_admin(&self, user_id: i64) -> ApiResult<()> {
        if self.is_admin() || self.id == user_id {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

/// Credentials pulled off a request before any async work happens.
#[derive(Debug, Default, Clone)]
pub struct Credentials {
    pub bearer: Option<String>,
    pub session: Option<String>,
}

impl Credentials {
    pub fn from_request(req: &HttpRequest) -> Self {
        let bearer = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| {
                let (scheme, token) = s.split_once(' ')?;
                scheme
                    .eq_ignore_ascii_case("bearer")
                    .then(|| token.trim().to_string())
            })
            .filter(|t| !t.is_empty());
        let session = req.cookie(SESSION_COOKIE).map(|c| c.value().to_string());
        Self { bearer, session }
    }
}

pub async fn load_user<'e, E>(exec: E, id: i64) -> ApiResult<Option<AuthUser>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, AuthUser>(
        "SELECT id, COALESCE(username, email) AS username, email, role FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(exec)
    .await
    .map_err(Into::into)
}

/// Resolve credentials to a user.
///
/// Returns `Ok(None)` when no credentials were supplied, and
/// `Err(Unauthorized)` when credentials were supplied but are invalid.
pub async fn authenticate(pool: &Db, creds: &Credentials) -> ApiResult<Option<AuthUser>> {
    if let Some(token) = &creds.bearer {
        return user_from_api_key(pool, token).await.map(Some);
    }
    if let Some(token) = &creds.session {
        return user_from_session(pool, token).await.map(Some);
    }
    Ok(None)
}

#[derive(FromRow)]
struct KeyRow {
    id: i64,
    user_id: i64,
    expires_at: Option<DateTime<Utc>>,
}

async fn user_from_api_key(pool: &Db, token: &str) -> ApiResult<AuthUser> {
    let key =
        sqlx::query_as::<_, KeyRow>("SELECT id, user_id, expires_at FROM api_keys WHERE key = ?")
            .bind(hash_token(token))
            .fetch_optional(pool)
            .await?
            .ok_or(ApiError::Unauthorized)?;

    if let Some(expires_at) = key.expires_at
        && expires_at <= Utc::now()
    {
        return Err(ApiError::Unauthorized);
    }

    let user = load_user(pool, key.user_id)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
        .bind(now())
        .bind(key.id)
        .execute(pool)
        .await?;

    Ok(user)
}

async fn user_from_session(pool: &Db, token: &str) -> ApiResult<AuthUser> {
    let user_id = sqlx::query_scalar::<_, i64>(
        "SELECT user_id FROM sessions WHERE id = ? AND expires_at > ?",
    )
    .bind(hash_token(token))
    .bind(now())
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::Unauthorized)?;

    load_user(pool, user_id)
        .await?
        .ok_or(ApiError::Unauthorized)
}

/// Create a session and return the (unhashed) token to place in the cookie.
pub async fn create_session(pool: &Db, user_id: i64, ttl: Duration) -> ApiResult<String> {
    let token = generate_secret();
    let created = Utc::now();
    sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(created.to_rfc3339())
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO sessions (id, user_id, created_at, expires_at) VALUES (?, ?, ?, ?)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(created.to_rfc3339())
        .bind((created + ttl).to_rfc3339())
        .execute(pool)
        .await?;
    Ok(token)
}

pub async fn delete_session(pool: &Db, token: &str) -> ApiResult<()> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(hash_token(token))
        .execute(pool)
        .await?;
    Ok(())
}

pub fn session_cookie(token: &str, ttl: Duration) -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE, token.to_string())
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::seconds(
            ttl.num_seconds(),
        ))
        .finish()
}

pub fn clear_session_cookie() -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, "");
    cookie.set_path("/");
    cookie.make_removal();
    cookie
}

type AuthFuture<T> = Pin<Box<dyn Future<Output = Result<T, ApiError>>>>;

fn pool_from_request(req: &HttpRequest) -> ApiResult<web::Data<Db>> {
    req.app_data::<web::Data<Db>>()
        .cloned()
        .ok_or_else(|| ApiError::Internal("database pool is not registered".into()))
}

impl FromRequest for AuthUser {
    type Error = ApiError;
    type Future = AuthFuture<AuthUser>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let pool = pool_from_request(req);
        let creds = Credentials::from_request(req);
        Box::pin(async move {
            let pool = pool?;
            authenticate(&pool, &creds)
                .await?
                .ok_or(ApiError::Unauthorized)
        })
    }
}

/// Like [`AuthUser`] but tolerates anonymous callers. Invalid credentials are
/// still rejected with 401 so a typo in an API key never silently downgrades
/// a request to guest access.
#[derive(Debug, Clone)]
pub struct OptionalAuth(pub Option<AuthUser>);

impl FromRequest for OptionalAuth {
    type Error = ApiError;
    type Future = AuthFuture<OptionalAuth>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let pool = pool_from_request(req);
        let creds = Credentials::from_request(req);
        Box::pin(async move {
            let pool = pool?;
            Ok(OptionalAuth(authenticate(&pool, &creds).await?))
        })
    }
}
