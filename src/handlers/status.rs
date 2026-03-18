use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};




#[get("/")]
pub async fn get_status() -> impl Responder {
   HttpResponse::Ok()
}
