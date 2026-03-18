use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

pub async fn status_handler() -> impl Responder {
   HttpResponse::Ok()
}
