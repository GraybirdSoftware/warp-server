use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


#[post("/")]
async fn files() -> impl Responder {
   HttpResponse::InternalServerError()
}


#[post("/json")]
async fn json() -> impl Responder {
   HttpResponse::InternalServerError()
}

