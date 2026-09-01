use actix_web::{HttpResponse, get, post, web};
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    db::{Db, like_contains},
    error::{ApiError, ApiResult},
    models::{core::SymbolMdl, pagination::Pagination, request::SymbolQuery},
};

const COLUMNS: &str = "id, created_at, name, class, modifier";

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &SymbolQuery) {
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

#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<SymbolQuery>) -> ApiResult<HttpResponse> {
    let page = Pagination::new(body.limit, body.page);

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM symbols WHERE 1=1");
    apply_filters(&mut count, &body);
    let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

    let mut select = QueryBuilder::new(format!("SELECT {COLUMNS} FROM symbols WHERE 1=1"));
    apply_filters(&mut select, &body);
    select
        .push(" ORDER BY name, id LIMIT ")
        .push_bind(page.limit)
        .push(" OFFSET ")
        .push_bind(page.offset());
    let items: Vec<SymbolMdl> = select.build_query_as().fetch_all(pool.get_ref()).await?;

    Ok(HttpResponse::Ok().json(page.wrap(items, total)))
}

#[get("/{id}")]
async fn get_symbol(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    let symbol =
        sqlx::query_as::<_, SymbolMdl>(&format!("SELECT {COLUMNS} FROM symbols WHERE id = ?"))
            .bind(*id)
            .fetch_optional(pool.get_ref())
            .await?
            .ok_or_else(|| ApiError::not_found("symbol"))?;
    Ok(HttpResponse::Ok().json(symbol))
}
