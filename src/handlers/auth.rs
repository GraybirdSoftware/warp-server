use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};



#[post("/login")]
pub async fn login() -> impl Responder {
   HttpResponse::InternalServerError()
}

#[post("/logout")]
pub async fn logout() -> impl Responder {
   HttpResponse::InternalServerError()
}

#[get("/o/callback")]
pub async fn get_callback() -> impl Responder {
   HttpResponse::InternalServerError()
}
