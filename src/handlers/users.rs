use actix_web::{HttpResponse, delete, get, patch, post, web};
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    auth::AuthUser,
    db::{self, Db, api_keys, cascade, like_contains, users},
    error::{ApiError, ApiResult},
    models::{
        core::{Role, UserInfo},
        pagination::Pagination,
        request::{ChangeRole, ChangeUsername, CreateApiKey, CreateUser, UserQuery},
    },
};

pub mod me;

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &UserQuery) {
    if let Some(name) = q.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if q.exact.unwrap_or(false) {
            qb.push(" AND COALESCE(username, email) = ")
                .push_bind(name.to_string());
        } else {
            qb.push(" AND COALESCE(username, email) LIKE ")
                .push_bind(like_contains(name))
                .push(" ESCAPE '\\'");
        }
    }
    if let Some(role) = q.role {
        qb.push(" AND role = ").push_bind(role.as_str());
    }
}

#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<UserQuery>) -> ApiResult<HttpResponse> {
    let page = Pagination::new(body.limit, body.page);

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM users WHERE 1=1");
    apply_filters(&mut count, &body);
    let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

    let mut select = QueryBuilder::new(format!(
        "SELECT {} FROM users WHERE 1=1",
        users::INFO_COLUMNS
    ));
    apply_filters(&mut select, &body);
    select
        .push(" ORDER BY id LIMIT ")
        .push_bind(page.limit)
        .push(" OFFSET ")
        .push_bind(page.offset());
    let items: Vec<UserInfo> = select.build_query_as().fetch_all(pool.get_ref()).await?;

    Ok(HttpResponse::Ok().json(page.wrap(items, total)))
}

#[get("/{id}")]
async fn get_user(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    let user = users::get_info(pool.get_ref(), *id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;
    Ok(HttpResponse::Ok().json(user))
}

// NOTE: every route below requires authentication.

#[tracing::instrument(name = "create_user", skip_all, fields(email = %request.email, by = %auth.username))]
#[post("")]
async fn create_user(
    pool: web::Data<Db>,
    auth: AuthUser,
    request: web::Json<CreateUser>,
) -> ApiResult<HttpResponse> {
    auth.require_admin()?;
    let mut conn = pool.acquire().await?;
    let user = users::create(
        &mut conn,
        &request.email,
        request.name.as_deref(),
        Role::User,
        users::UsernamePolicy::Strict,
    )
    .await?;
    tracing::info!(user_id = user.id, "user created");
    Ok(HttpResponse::Created().json(user))
}

#[delete("/{id}")]
async fn delete_user(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    auth.require_admin()?;
    let mut tx = pool.begin().await?;
    if !db::user_exists(&mut *tx, *id).await? {
        return Err(ApiError::not_found("user"));
    }
    cascade::delete_user(&mut tx, *id).await?;
    tx.commit().await?;
    tracing::info!(user_id = *id, by = %auth.username, "user deleted");
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{id}/keys")]
async fn get_keys(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    auth.require_self_or_admin(*id)?;
    if !db::user_exists(pool.get_ref(), *id).await? {
        return Err(ApiError::not_found("user"));
    }
    Ok(HttpResponse::Ok().json(api_keys::list(pool.get_ref(), *id).await?))
}

#[post("/{id}/keys")]
async fn create_key(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<i64>,
    body: web::Json<CreateApiKey>,
) -> ApiResult<HttpResponse> {
    auth.require_self_or_admin(*id)?;
    if !db::user_exists(pool.get_ref(), *id).await? {
        return Err(ApiError::not_found("user"));
    }
    let key = api_keys::create(pool.get_ref(), *id, &body.name).await?;
    Ok(HttpResponse::Created().json(key))
}

#[patch("/{id}/role")]
async fn change_role(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<i64>,
    body: web::Json<ChangeRole>,
) -> ApiResult<HttpResponse> {
    auth.require_admin()?;
    let updated = sqlx::query("UPDATE users SET role = ? WHERE id = ?")
        .bind(body.role.as_str())
        .bind(*id)
        .execute(pool.get_ref())
        .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::not_found("user"));
    }
    tracing::info!(user_id = *id, role = body.role.as_str(), by = %auth.username, "role changed");
    let user = users::get_info(pool.get_ref(), *id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;
    Ok(HttpResponse::Ok().json(user))
}

#[patch("/{id}/username")]
async fn change_username(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<i64>,
    body: web::Json<ChangeUsername>,
) -> ApiResult<HttpResponse> {
    auth.require_self_or_admin(*id)?;
    let username = body.username.trim();
    users::validate_username(username)?;
    if !db::user_exists(pool.get_ref(), *id).await? {
        return Err(ApiError::not_found("user"));
    }
    if users::username_taken(pool.get_ref(), username, Some(*id)).await? {
        return Err(ApiError::Conflict("username already taken".into()));
    }
    sqlx::query("UPDATE users SET username = ? WHERE id = ?")
        .bind(username)
        .bind(*id)
        .execute(pool.get_ref())
        .await?;
    let user = users::get_info(pool.get_ref(), *id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;
    Ok(HttpResponse::Ok().json(user))
}
