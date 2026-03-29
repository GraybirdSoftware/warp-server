use actix_web::{HttpResponse, Responder, delete, get, patch, post, web};
use chrono::Utc;
use sqlx::{Pool, Sqlite};

use crate::models::{
    core::{Role, User},
    request::CreateUser,
};

pub mod me;

#[post("/query")]
async fn query() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}")]
async fn get_user(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

// NOTE: ALL ROUTES BELOW THIS ARE AUTHENTICATED
#[tracing::instrument(
    name = "create_user",
    fields(
        user_email = ?request.email,
        user_name = ?request.name
    ),
    err
)]
#[post("")]
async fn create_user(
    pool: web::Data<Pool<Sqlite>>,
    request: web::Json<CreateUser>,
) -> Result<HttpResponse, actix_web::Error> {
    let conn = pool.get_ref();

    let row = sqlx::query!(
        "SELECT 1 AS present FROM users WHERE email = ? LIMIT 1",
        request.email
    )
    .fetch_optional(conn)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if row.is_none() {
        tracing::debug!("user, is none");
        let created_at = Utc::now().to_rfc3339();
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, email, role, created_at)
            VALUES (?, ?, ?, ?)
            RETURNING id, username, email, role, created_at
            "#,
            request.name,
            request.email,
            Role::User,
            created_at
        )
        .fetch_one(conn)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

        return Ok(HttpResponse::Ok().json(user));
    }

    Ok(HttpResponse::GatewayTimeout().finish())
}

#[delete("/{id}")]
async fn delete_user(pool: web::Data<Pool<Sqlite>>, id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/keys")]
async fn get_keys(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[post("/{id}/keys")]
async fn create_key(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[patch("/{id}/role")]
async fn change_role(pool: web::Data<Pool<Sqlite>>, id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[patch("/{id}/username")]
async fn change_username(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}
