use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

async fn files() -> impl Responder {
   HttpResponse::InternalServerError()
}

