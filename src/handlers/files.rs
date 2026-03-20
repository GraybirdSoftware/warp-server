use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


#[post("/")]
async fn files() -> impl Responder {
   HttpResponse::NotImplemented()
}


#[post("/json")]
async fn json() -> impl Responder {
   HttpResponse::NotImplemented()
}

