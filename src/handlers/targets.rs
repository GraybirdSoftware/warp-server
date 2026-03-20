use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


#[post("/query")]
async fn query() -> impl Responder {
   HttpResponse::NotImplemented()
}

#[get("/{id}")]
async fn get_target(id: web::Path<(u32,)>) -> impl Responder {
   HttpResponse::NotImplemented()
}

