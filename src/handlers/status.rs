use actix_web::{HttpResponse, Responder, get};

use crate::models::response::StatusResponse;

#[get("")]
pub async fn get_status() -> impl Responder {
    HttpResponse::Ok().json(StatusResponse {
        status: "OK".to_string(),
    })
}
