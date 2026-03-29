use actix_web::{HttpResponse, Responder, delete, get, post, web};

#[post("/")]
async fn create_source() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[post("/query")]
async fn query() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}")]
async fn get_source(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[post("/{id}")]
async fn update_source(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[delete("/{id}")]
async fn delete_source(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/tags")]
async fn get_source_tags(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/users")]
async fn get_source_users(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}
