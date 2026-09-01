use actix_web::{HttpResponse, delete, get, post, web};
use sqlx::{QueryBuilder, Sqlite, SqliteConnection};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{self, Db, cascade, like_contains, now, uid},
    error::{ApiError, ApiResult},
    models::{
        core::{SourceMdl, SourceTagMdl, SourceUserMdl},
        pagination::Pagination,
        request::{CreateSource, PutSource, SourceQuery},
    },
};

const COLUMNS: &str = "id, created_at, name";

async fn load_source(pool: &Db, id: &str) -> ApiResult<SourceMdl> {
    sqlx::query_as::<_, SourceMdl>("SELECT id, created_at, name FROM source WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("source"))
}

/// Members and admins may change a source.
async fn require_member(pool: &Db, auth: &AuthUser, source_id: &str) -> ApiResult<()> {
    if auth.is_admin() || db::is_source_member(pool, source_id, auth.id).await? {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}

async fn replace_members(
    conn: &mut SqliteConnection,
    source_id: &str,
    user_ids: &[i64],
) -> ApiResult<()> {
    for user_id in user_ids {
        if !db::user_exists(&mut *conn, *user_id).await? {
            return Err(ApiError::NotFound(format!("user {user_id} not found")));
        }
    }
    sqlx::query("DELETE FROM source_users WHERE source_id = ?")
        .bind(source_id)
        .execute(&mut *conn)
        .await?;
    for user_id in user_ids {
        sqlx::query("INSERT OR IGNORE INTO source_users (source_id, user_id) VALUES (?, ?)")
            .bind(source_id)
            .bind(*user_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

fn validate_name(name: &str) -> ApiResult<&str> {
    let name = name.trim();
    if name.is_empty() || name.len() > 256 {
        return Err(ApiError::bad_request(
            "source name must be between 1 and 256 characters",
        ));
    }
    Ok(name)
}

#[post("")]
async fn create_source(
    pool: web::Data<Db>,
    auth: AuthUser,
    body: web::Json<CreateSource>,
) -> ApiResult<HttpResponse> {
    let name = validate_name(&body.name)?;
    let id = uid(&Uuid::new_v4());

    // Non-admins are always members of what they create; an empty list means
    // "just me" for everyone.
    let mut members = body.user_ids.clone();
    if members.is_empty() || !auth.is_admin() {
        members.push(auth.id);
    }
    members.sort_unstable();
    members.dedup();

    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO source (id, created_at, name) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(now())
        .bind(name)
        .execute(&mut *tx)
        .await?;
    replace_members(&mut tx, &id, &members).await?;
    tx.commit().await?;

    tracing::info!(source = %id, %name, by = %auth.username, "source created");
    Ok(HttpResponse::Ok().json(load_source(pool.get_ref(), &id).await?))
}

fn apply_filters(qb: &mut QueryBuilder<'_, Sqlite>, q: &SourceQuery) {
    if let Some(name) = q.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if q.exact.unwrap_or(false) {
            qb.push(" AND name = ").push_bind(name.to_string());
        } else {
            qb.push(" AND name LIKE ")
                .push_bind(like_contains(name))
                .push(" ESCAPE '\\'");
        }
    }
    if let Some(user_id) = q.user_id {
        qb.push(" AND EXISTS (SELECT 1 FROM source_users su WHERE su.source_id = source.id AND su.user_id = ")
            .push_bind(user_id)
            .push(")");
    }
}

#[post("/query")]
async fn query(pool: web::Data<Db>, body: web::Json<SourceQuery>) -> ApiResult<HttpResponse> {
    let page = Pagination::new(body.limit, body.page);

    let mut count = QueryBuilder::new("SELECT COUNT(*) FROM source WHERE 1=1");
    apply_filters(&mut count, &body);
    let total: i64 = count.build_query_scalar().fetch_one(pool.get_ref()).await?;

    let mut select = QueryBuilder::new(format!("SELECT {COLUMNS} FROM source WHERE 1=1"));
    apply_filters(&mut select, &body);
    select
        .push(" ORDER BY created_at, id LIMIT ")
        .push_bind(page.limit)
        .push(" OFFSET ")
        .push_bind(page.offset());
    let items: Vec<SourceMdl> = select.build_query_as().fetch_all(pool.get_ref()).await?;

    Ok(HttpResponse::Ok().json(page.wrap(items, total)))
}

#[get("/{id}")]
async fn get_source(pool: web::Data<Db>, id: web::Path<Uuid>) -> ApiResult<HttpResponse> {
    Ok(HttpResponse::Ok().json(load_source(pool.get_ref(), &uid(&id)).await?))
}

#[post("/{id}")]
async fn update_source(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<Uuid>,
    body: web::Json<PutSource>,
) -> ApiResult<HttpResponse> {
    let id = uid(&id);
    load_source(pool.get_ref(), &id).await?;
    require_member(pool.get_ref(), &auth, &id).await?;
    if body.tags.is_some() && !auth.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let mut tx = pool.begin().await?;
    if let Some(name) = body.name.as_deref() {
        let name = validate_name(name)?;
        sqlx::query("UPDATE source SET name = ? WHERE id = ?")
            .bind(name)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
    }
    if let Some(user_ids) = &body.user_ids {
        replace_members(&mut tx, &id, user_ids).await?;
    }
    if let Some(tags) = &body.tags {
        sqlx::query("DELETE FROM source_tags WHERE source_id = ?")
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        for tag in tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            sqlx::query("INSERT OR IGNORE INTO source_tags (source_id, tag) VALUES (?, ?)")
                .bind(&id)
                .bind(tag)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;

    tracing::info!(source = %id, by = %auth.username, "source updated");
    Ok(HttpResponse::Ok().json(load_source(pool.get_ref(), &id).await?))
}

#[delete("/{id}")]
async fn delete_source(
    pool: web::Data<Db>,
    auth: AuthUser,
    id: web::Path<Uuid>,
) -> ApiResult<HttpResponse> {
    let id = uid(&id);
    load_source(pool.get_ref(), &id).await?;
    require_member(pool.get_ref(), &auth, &id).await?;

    let mut tx = pool.begin().await?;
    cascade::delete_source(&mut tx, &id).await?;
    tx.commit().await?;

    tracing::info!(source = %id, by = %auth.username, "source deleted");
    Ok(HttpResponse::NoContent().finish())
}

#[get("/{id}/tags")]
async fn get_source_tags(pool: web::Data<Db>, id: web::Path<Uuid>) -> ApiResult<HttpResponse> {
    let id = uid(&id);
    load_source(pool.get_ref(), &id).await?;
    let tags = sqlx::query_as::<_, SourceTagMdl>(
        "SELECT source_id, tag FROM source_tags WHERE source_id = ? ORDER BY tag",
    )
    .bind(&id)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(tags))
}

#[get("/{id}/users")]
async fn get_source_users(pool: web::Data<Db>, id: web::Path<Uuid>) -> ApiResult<HttpResponse> {
    let id = uid(&id);
    load_source(pool.get_ref(), &id).await?;
    let users = sqlx::query_as::<_, SourceUserMdl>(
        "SELECT source_id, user_id FROM source_users WHERE source_id = ? ORDER BY user_id",
    )
    .bind(&id)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(users))
}
