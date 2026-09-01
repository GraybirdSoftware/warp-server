//! Targets (platform + architecture pairs) are created implicitly when files
//! are pushed; these routes only look them up.

use actix_web::{HttpResponse, get, post, web};
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    db::Db,
    error::{ApiError, ApiResult},
    models::{core::TargetMdl, request::TargetQuery},
};

const SELECT: &str = "SELECT id, created_at, NULLIF(platform, '') AS platform, NULLIF(arch, '') AS arch FROM targets WHERE 1=1";

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &TargetQuery) {
    if let Some(platform) = q.platform.as_deref().map(str::trim) {
        qb.push(" AND platform = ").push_bind(platform.to_string());
    }
    if let Some(arch) = q.arch.as_deref().map(str::trim) {
        qb.push(" AND arch = ").push_bind(arch.to_string());
    }
}

async fn find(pool: &Db, q: &TargetQuery) -> ApiResult<Vec<TargetMdl>> {
    let mut select = QueryBuilder::new(SELECT);
    apply_filters(&mut select, q);
    select.push(" ORDER BY id");
    select
        .build_query_as()
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

/// All targets matching the given platform and/or architecture.
#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<TargetQuery>) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(find(pool.get_ref(), &body).await?))
}

/// Same lookup as a GET; answers with the first matching id as `text/plain`.
#[get("/query")]
async fn get_query(
    pool: web::Data<Db>,
    params: web::Query<TargetQuery>,
) -> ApiResult<HttpResponse> {
    let target = find(pool.get_ref(), &params)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::not_found("target"))?;
    Ok(HttpResponse::Ok()
        .content_type("text/plain")
        .body(target.id.to_string()))
}

#[get("/{id}")]
async fn get_target(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    let target = sqlx::query_as::<_, TargetMdl>(&format!("{SELECT} AND id = ?"))
        .bind(*id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("target"))?;
    Ok(HttpResponse::Ok().json(target))
}
