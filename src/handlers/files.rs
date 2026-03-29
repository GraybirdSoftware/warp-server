use actix_web::{HttpResponse, Responder, post};

#[post("/")]
async fn files() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[post("/json")]
async fn json() -> impl Responder {
    HttpResponse::NotImplemented()
}
