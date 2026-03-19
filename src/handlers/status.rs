use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use serde_json::json;




#[get("")]
pub async fn get_status() -> impl Responder {
   HttpResponse::Ok().json(json!({
      "status": "OK"
   }))
}
