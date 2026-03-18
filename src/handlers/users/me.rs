use actix_web::{HttpResponse, Responder, get, post};

#[get("/me")]
async fn get_user() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/me/keys")]
async fn get_keys() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[post("/me/keys")]
async fn create_key() -> impl Responder {
    HttpResponse::NotImplemented()
}
