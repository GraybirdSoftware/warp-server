use actix_web::{HttpResponse, get, post, web};
use sqlx::{QueryBuilder, Sqlite};
use uuid::Uuid;

use crate::{
    db::{
        Db, json_strings, like_contains, uid,
        warpfile::{self, TypeBlob},
    },
    error::{ApiError, ApiResult},
    models::{
        core::{Format, TypeMdl},
        pagination::Pagination,
        request::{TypeIds, TypeQuery},
    },
};

const COLUMNS: &str = "id, created_at, commit_id, source_id, name, data";

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &TypeQuery) {
    if let Some(name) = q.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if q.exact.unwrap_or(false) {
            qb.push(" AND name = ").push_bind(name.to_string());
        } else {
            qb.push(" AND name LIKE ")
                .push_bind(like_contains(name))
                .push(" ESCAPE '\\'");
        }
    }
}

/// Query types as JSON (paginated) or as a WARP file (`format: "flatbuffer"`).
#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<TypeQuery>) -> ApiResult<HttpResponse> {
    let page = Pagination::new(body.limit, body.page);

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM types WHERE 1=1");
    apply_filters(&mut count, &body);
    let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

    let mut select = QueryBuilder::new(format!("SELECT {COLUMNS} FROM types WHERE 1=1"));
    apply_filters(&mut select, &body);
    select
        .push(" ORDER BY name, id LIMIT ")
        .push_bind(page.limit)
        .push(" OFFSET ")
        .push_bind(page.offset());

    match body.format.unwrap_or_default() {
        Format::Json => {
            let items: Vec<TypeMdl> = select.build_query_as().fetch_all(pool.get_ref()).await?;
            Ok(HttpResponse::Ok().json(page.wrap(items, total)))
        }
        Format::Flatbuffer => {
            let rows: Vec<TypeBlob> = select.build_query_as().fetch_all(pool.get_ref()).await?;
            Ok(warpfile::octet_stream(warpfile::type_file(rows)))
        }
    }
}

/// Batch variant of `GET /{id}/data`: a WARP file with the requested types.
#[post("/data")]
async fn get_types_data(pool: web::Data<Db>, body: web::Json<TypeIds>) -> ApiResult<HttpResponse> {
    let rows = sqlx::query_as::<_, TypeBlob>(
        "SELECT id, data FROM types WHERE id IN (SELECT value FROM json_each(?)) ORDER BY id",
    )
    .bind(json_strings(body.ids.iter().map(uid)))
    .fetch_all(pool.get_ref())
    .await?;
    Ok(warpfile::octet_stream(warpfile::type_file(rows)))
}

#[get("/{id}")]
async fn get_types(pool: web::Data<Db>, id: web::Path<Uuid>) -> ApiResult<HttpResponse> {
    let ty = sqlx::query_as::<_, TypeMdl>(&format!("SELECT {COLUMNS} FROM types WHERE id = ?"))
        .bind(uid(&id))
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("type"))?;
    Ok(HttpResponse::Ok().json(ty))
}

/// The raw WARP `Type` flatbuffer.
#[get("/{id}/data")]
async fn get_types_data_by_id(pool: web::Data<Db>, id: web::Path<Uuid>) -> ApiResult<HttpResponse> {
    let data = sqlx::query_scalar::<_, Vec<u8>>("SELECT data FROM types WHERE id = ?")
        .bind(uid(&id))
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("type"))?;
    Ok(warpfile::octet_stream(data))
}
