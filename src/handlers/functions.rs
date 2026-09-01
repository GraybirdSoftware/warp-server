use std::collections::BTreeMap;

use actix_web::{HttpResponse, get, post, web};
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    db::{
        Db, json_ints, json_strings, uid,
        warpfile::{self, FUNCTION_BLOB_SELECT, FunctionBlob},
    },
    error::{ApiError, ApiResult},
    models::{
        core::{
            Format, FunctionCommentMdl, FunctionConstraintMdl, FunctionMdl, SymbolMdl, TypeMdl,
        },
        pagination::Pagination,
        request::{FunctionIds, FunctionQuery},
    },
};

const COLUMNS: &str =
    "f.id, f.created_at, f.commit_id, f.target_id, f.source_id, f.guid, f.symbol_id, f.type_id";

/// Shared WHERE clause; the functions table is aliased `f`.
/// Empty lists are treated as "no filter".
fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &FunctionQuery) {
    if let Some(commit_id) = q.commit_id {
        qb.push(" AND f.commit_id = ").push_bind(commit_id);
    }
    if let Some(source_id) = &q.source_id {
        qb.push(" AND f.source_id = ").push_bind(uid(source_id));
    }
    if let Some(target_id) = q.target_id {
        qb.push(" AND f.target_id = ").push_bind(target_id);
    }
    if let Some(symbol_id) = q.symbol_id {
        qb.push(" AND f.symbol_id = ").push_bind(symbol_id);
    }
    if let Some(guids) = q.guids.as_ref().filter(|g| !g.is_empty()) {
        qb.push(" AND f.guid IN (SELECT value FROM json_each(")
            .push_bind(json_strings(guids.iter().map(uid)))
            .push("))");
    }
    if let Some(constraints) = q.constraints.as_ref().filter(|c| !c.is_empty()) {
        qb.push(
            " AND EXISTS (SELECT 1 FROM function_constraints fc WHERE fc.function_id = f.id \
             AND fc.guid IN (SELECT value FROM json_each(",
        )
        .push_bind(json_strings(constraints.iter().map(uid)))
        .push(")))");
    }
    if let Some(tags) = q.source_tags.as_ref().filter(|t| !t.is_empty()) {
        qb.push(
            " AND EXISTS (SELECT 1 FROM source_tags st WHERE st.source_id = f.source_id \
             AND st.tag IN (SELECT value FROM json_each(",
        )
        .push_bind(json_strings(tags))
        .push(")))");
    }
}

fn push_page(qb: &mut QueryBuilder<'_, Sqlite>, page: &Pagination) {
    qb.push(" ORDER BY f.id LIMIT ")
        .push_bind(page.limit)
        .push(" OFFSET ")
        .push_bind(page.offset());
}

/// Query functions as JSON (paginated) or as a WARP file (`format: "flatbuffer"`).
#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<FunctionQuery>) -> ApiResult<HttpResponse> {
    let page = Pagination::new(body.limit, body.page);

    match body.format.unwrap_or_default() {
        Format::Json => {
            let mut count = QueryBuilder::new("SELECT COUNT(*) FROM functions f WHERE 1=1");
            apply_filters(&mut count, &body);
            let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

            let mut select =
                QueryBuilder::new(format!("SELECT {COLUMNS} FROM functions f WHERE 1=1"));
            apply_filters(&mut select, &body);
            push_page(&mut select, &page);
            let items: Vec<FunctionMdl> = select.build_query_as().fetch_all(pool.get_ref()).await?;
            Ok(HttpResponse::Ok().json(page.wrap(items, total)))
        }
        Format::Flatbuffer => {
            let mut select = QueryBuilder::new(format!("{FUNCTION_BLOB_SELECT} WHERE 1=1"));
            apply_filters(&mut select, &body);
            push_page(&mut select, &page);
            let rows: Vec<FunctionBlob> = select.build_query_as().fetch_all(pool.get_ref()).await?;
            Ok(warpfile::octet_stream(warpfile::function_file(rows)))
        }
    }
}

/// Which sources know which of the requested GUIDs: `{ source_id: [guid, ...] }`.
#[post("/query/source")]
async fn query_source(
    pool: web::Data<Db>,
    body: web::Json<FunctionQuery>,
) -> ApiResult<HttpResponse> {
    let mut select =
        QueryBuilder::new("SELECT DISTINCT f.source_id, f.guid FROM functions f WHERE 1=1");
    apply_filters(&mut select, &body);
    select.push(" ORDER BY f.source_id, f.guid");
    if let Some(limit) = body.limit {
        select.push(" LIMIT ").push_bind(limit.max(0));
    }
    let rows: Vec<(String, String)> = select.build_query_as().fetch_all(pool.get_ref()).await?;

    let mut mapped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (source_id, guid) in rows {
        mapped.entry(source_id).or_default().push(guid);
    }
    Ok(HttpResponse::Ok().json(mapped))
}

/// Batch variant of `GET /{id}/data`: a WARP file with the requested functions.
#[post("/data")]
async fn get_functions_data(
    pool: web::Data<Db>,
    body: web::Json<FunctionIds>,
) -> ApiResult<HttpResponse> {
    let rows = sqlx::query_as::<_, FunctionBlob>(&format!(
        "{FUNCTION_BLOB_SELECT} WHERE f.id IN (SELECT value FROM json_each(?)) ORDER BY f.id"
    ))
    .bind(json_ints(&body.ids))
    .fetch_all(pool.get_ref())
    .await?;
    Ok(warpfile::octet_stream(warpfile::function_file(rows)))
}

async fn require_function(pool: &Db, id: i64) -> ApiResult<()> {
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM functions WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    if exists > 0 {
        Ok(())
    } else {
        Err(ApiError::not_found("function"))
    }
}

#[get("/{id}")]
async fn get_function(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    let function = sqlx::query_as::<_, FunctionMdl>(&format!(
        "SELECT {COLUMNS} FROM functions f WHERE f.id = ?"
    ))
    .bind(*id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("function"))?;
    Ok(HttpResponse::Ok().json(function))
}

#[get("/{id}/comments")]
async fn get_function_comments(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    require_function(pool.get_ref(), *id).await?;
    let comments = sqlx::query_as::<_, FunctionCommentMdl>(
        "SELECT id, function_id, text, byte_offset FROM function_comments WHERE function_id = ? ORDER BY byte_offset, id",
    )
    .bind(*id)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(comments))
}

#[get("/{id}/constraints")]
async fn get_function_constraints(
    pool: web::Data<Db>,
    id: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    require_function(pool.get_ref(), *id).await?;
    let constraints = sqlx::query_as::<_, FunctionConstraintMdl>(
        "SELECT id, function_id, guid, byte_offset FROM function_constraints WHERE function_id = ? ORDER BY id",
    )
    .bind(*id)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(constraints))
}

/// The raw WARP `Function` flatbuffer.
#[get("/{id}/data")]
async fn get_function_data(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    let data = sqlx::query_scalar::<_, Vec<u8>>("SELECT data FROM functions WHERE id = ?")
        .bind(*id)
        .fetch_optional(pool.get_ref())
        .await?
        .ok_or_else(|| ApiError::not_found("function"))?;
    Ok(warpfile::octet_stream(data))
}

#[get("/{id}/symbol")]
async fn get_function_symbol(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    require_function(pool.get_ref(), *id).await?;
    let symbol = sqlx::query_as::<_, SymbolMdl>(
        "SELECT s.id, s.created_at, s.name, s.class, s.modifier FROM symbols s
         JOIN functions f ON f.symbol_id = s.id WHERE f.id = ?",
    )
    .bind(*id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::not_found("symbol"))?;
    Ok(HttpResponse::Ok().json(symbol))
}

#[get("/{id}/type")]
async fn get_function_type(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    require_function(pool.get_ref(), *id).await?;
    let ty = sqlx::query_as::<_, TypeMdl>(
        "SELECT t.id, t.created_at, t.commit_id, t.source_id, t.name, t.data FROM types t
         JOIN functions f ON f.type_id = t.id WHERE f.id = ?",
    )
    .bind(*id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| ApiError::NotFound("function has no type".into()))?;
    Ok(HttpResponse::Ok().json(ty))
}
