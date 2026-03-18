use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};

async fn login() -> impl Responder {
   HttpResponse::InternalServerError()
}

async fn logout() -> impl Responder {
   HttpResponse::InternalServerError()
}

async fn callback() -> impl Responder {
   HttpResponse::InternalServerError()
}
