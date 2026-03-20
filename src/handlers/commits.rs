use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};



#[post("/query")]
async fn query() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}")]
async fn get_commit(id: web::Path<(u32,)>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[delete("/{id}")]
async fn delete_commit(id: web::Path<(u32,)>) -> impl Responder {
    HttpResponse::NotImplemented()
}