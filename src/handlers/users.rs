use actix_web::{HttpResponse, Responder, delete, get, patch, post, web};
use chrono::Utc;
use sqlx::{Pool, Sqlite};

use crate::models::{core::Role, request::CreateUser};

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
#[post("/")]
async fn create_user(pool: web::Data<Pool<Sqlite>>, request: web::Json<CreateUser>) -> Result<HttpResponse, actix_web::Error> {
    let conn = pool.get_ref();

    let row = sqlx::query("SELECT 1 FROM users WHERE email = ? LIMIT 1")
        .bind(&request.email)
        .fetch_optional(conn)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;


    if !row.is_some() { 
        let id = sqlx::query(
                "INSERT INTO users (username, email, role, created_at
                VALUES (?1, ?2, ?3, ?4)")
            .bind(&request.name)
            .bind(&request.email)
            .bind(Role::User)
            .bind(Utc::now().to_rfc3339());

        //TODO; Return response of userdata in json
    }
    
    Ok(HttpResponse::InternalServerError().finish())

}

#[delete("/{id}")]
async fn delete_user(id: web::Path<u32>) -> impl Responder {
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
async fn change_role(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[patch("/{id}/username")]
async fn change_username(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}
