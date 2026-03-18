use actix_web::{HttpResponse, Responder, delete, get, patch, post, web};

mod me;

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
async fn create_user() -> impl Responder {
    HttpResponse::NotImplemented()
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
