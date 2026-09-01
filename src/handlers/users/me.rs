use actix_web::{HttpResponse, get, post, web};

use crate::{
    auth::AuthUser,
    db::{Db, api_keys, users},
    error::{ApiError, ApiResult},
    models::request::CreateApiKey,
};

#[get("")]
async fn get_user(pool: web::Data<Db>, auth: AuthUser) -> ApiResult<HttpResponse> {
    let user = users::get_mdl(pool.get_ref(), auth.id)
        .await?
        .ok_or_else(|| ApiError::not_found("user"))?;
    Ok(HttpResponse::Ok().json(user))
}

#[get("/keys")]
async fn get_keys(pool: web::Data<Db>, auth: AuthUser) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(api_keys::list(pool.get_ref(), auth.id).await?))
}

#[post("/keys")]
async fn create_key(
    pool: web::Data<Db>,
    auth: AuthUser,
    body: web::Json<CreateApiKey>,
) -> ApiResult<HttpResponse> {
    let key = api_keys::create(pool.get_ref(), auth.id, &body.name).await?;
    Ok(HttpResponse::Created().json(key))
}
