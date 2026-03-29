use actix_web::{HttpResponse, Responder, get};
use serde_json::json;

#[tracing::instrument(name = "status")]
#[get("")]
pub async fn get_status() -> impl Responder {
    tracing::info!("OK");
    HttpResponse::Ok().json(json!({
       "status": "OK"
    }))
}
