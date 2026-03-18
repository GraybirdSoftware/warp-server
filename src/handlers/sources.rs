use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


#[post("/")]
async fn create_source() -> impl Responder {
   HttpResponse::InternalServerError()
}

#[post("/query")]
async fn query() -> impl Responder {
   HttpResponse::InternalServerError()
}

#[get("/{id}")]
async fn get_source(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}

#[post("/{id}")]
async fn update_source(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}

#[delete("/{id}")]
async fn delete_source(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}

#[get("/{id}/tags")]
async fn get_source_tags(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}

#[get("/{id}/users")]
async fn get_source_users(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}