use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


async fn query() -> impl Responder {
   HttpResponse::InternalServerError()
}
async fn get_commit() -> impl Responder {
   HttpResponse::InternalServerError()
}

async fn delete_commit() -> impl Responder {
   HttpResponse::InternalServerError()
}
