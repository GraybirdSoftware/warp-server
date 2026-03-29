use actix_web::{HttpResponse, Responder, get, post, web};

#[post("/query")]
async fn query() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[post("/query/source")]
async fn query_source() -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}")]
async fn get_function(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/comments")]
async fn get_function_comments(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/constraints")]
async fn get_function_constraints(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/data")]
async fn get_function_data(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/symbol")]
async fn get_function_symbol(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}

#[get("/{id}/type")]
async fn get_function_type(id: web::Path<u32>) -> impl Responder {
    HttpResponse::NotImplemented()
}
