use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};


#[get("/")]
async fn search() -> impl Responder {
   HttpResponse::NotImplemented()
}