use actix_web::{HttpResponse, Responder, get, post, web};

#[post("/query")]
async fn query() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}")]
async fn get_types(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/data")]
async fn get_types_data(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}
