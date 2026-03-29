use actix_web::{HttpResponse, Responder, get};

#[get("/")]
async fn search() -> impl Responder {
    HttpResponse::NotImplemented()
}
