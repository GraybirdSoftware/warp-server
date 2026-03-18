use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


#[post("/query")]
async fn query() -> impl Responder {
   HttpResponse::InternalServerError()
}

#[get("/{id}")]
async fn get_types(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}

#[get("/{id}/data")]
async fn get_types_data(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::InternalServerError()
}

