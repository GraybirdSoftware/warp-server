use actix_web::{HttpResponse, delete, get, post, web};
use sqlx::{QueryBuilder, Sqlite};

use crate::{
    auth::AuthUser,
    db::{self, Db, cascade, uid},
    error::{ApiError, ApiResult},
    models::{core::CommitMdl, pagination::Pagination, request::CommitQuery},
};

const COLUMNS: &str = "id, created_at, description, name, source_id, user_id";

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &CommitQuery) {
    if let Some(source_id) = &q.source_id {
        qb.push(" AND source_id = ").push_bind(uid(source_id));
    }
    if let Some(user_id) = q.user_id {
        qb.push(" AND user_id = ").push_bind(user_id);
    }
}

#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<CommitQuery>) -> ApiResult<HttpResponse> {
    let page = Pagination::new(body.limit, body.page);

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM commits WHERE 1=1");
    apply_filters(&mut count, &body);
    let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

    let mut select = QueryBuilder::new(format!("SELECT {COLUMNS} FROM commits WHERE 1=1"));
    apply_filters(&mut select, &body);
    select
        .push(" ORDER BY id DESC LIMIT ")
        .push_bind(page.limit)
        .push(" OFFSET ")
        .push_bind(page.offset());
    let items: Vec<CommitMdl> = select.build_query_as().fetch_all(pool.get_ref()).await?;

    Ok(HttpResponse::Ok().json(page.wrap(items, total)))
}

async fn load_commit(pool: &Db, id: i64) -> ApiResult<CommitMdl> {
    sqlx::query_as::<_, CommitMdl>(
        "SELECT id, created_at, description, name, source_id, user_id FROM commits WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::not_found("commit"))
}

#[get("/{id}")]
async fn get_commit(pool: web::Data<Db>, id: web::Path<i64>) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(load_commit(pool.get_ref(), *id).await?))
}

/// The commit's author, members of its source, and admins may delete it.
#[delete("/{id}")]
async fn delete_commit(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let commit = load_commit(pool.get_ref(), *id).await?;
    let source_id = commit.source_id.to_string();
    let allowed = auth.is_admin()
        || commit.user_id == Some(auth.id)
        || db::is_source_member(pool.get_ref(), &source_id, auth.id).await?;
    if !allowed {
        return Err(ApiError::Forbidden);
    }

    let mut tx = pool.begin().await?;
    cascade::delete_commit(&mut tx, *id).await?;
    tx.commit().await?;

    tracing::info!(commit = *id, by = %auth.username, "commit deleted");
    Ok(HttpResponse::Ok().finish())
}
