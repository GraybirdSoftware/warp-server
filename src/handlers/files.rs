//! Pushing `.warp` files. This is how data gets into the server.

use actix_multipart::form::{MultipartForm, bytes::Bytes, text::Text};
use actix_web::{HttpResponse, post, web};
use base64::Engine;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{self, Db, ingest, uid},
    error::{ApiError, ApiResult},
    models::{request::UploadFileJson, response::PushResponse},
};

/// Maximum size of a decoded `.warp` upload.
pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, MultipartForm)]
pub struct UploadForm {
    #[multipart(limit = "64MiB")]
    pub file: Bytes,
    pub name: Text<String>,
    pub source: Text<String>,
    pub description: Option<Text<String>>,
}

async fn push(
    pool: &Db,
    auth: &AuthUser,
    source_id: Uuid,
    name: &str,
    description: Option<&str>,
    bytes: &[u8],
) -> ApiResult<HttpResponse> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name must not be empty"));
    }
    if bytes.len() > MAX_FILE_BYTES {
        return Err(ApiError::bad_request("file exceeds the 64 MiB limit"));
    }
    let source = uid(&source_id);
    if !db::source_exists(pool, &source).await? {
        return Err(ApiError::not_found("source"));
    }
    if !auth.is_admin() && !db::is_source_member(pool, &source, auth.id).await? {
        return Err(ApiError::Forbidden);
    }

    let summary =
        ingest::ingest_warp_file(pool, source_id, auth.id, name, description, bytes).await?;
    tracing::info!(
        commit = summary.commit_id,
        source = %source,
        chunks = summary.chunks,
        functions = summary.functions,
        types = summary.types,
        by = %auth.username,
        "file pushed"
    );
    Ok(HttpResponse::Ok().json(PushResponse {
        commit_id: summary.commit_id,
    }))
}

/// `multipart/form-data` upload with fields `file`, `name`, `source`
/// and optionally `description`.
#[post("")]
async fn push_file_multipart(
    pool: web::Data<Db>,
    auth: AuthUser,
    MultipartForm(form): MultipartForm<UploadForm>,
) -> ApiResult<HttpResponse> {
    let source_id = Uuid::parse_str(form.source.0.trim())
        .map_err(|_| ApiError::bad_request("source must be a UUID"))?;
    push(
        pool.get_ref(),
        &auth,
        source_id,
        &form.name.0,
        form.description.as_ref().map(|d| d.0.as_str()),
        &form.file.data,
    )
    .await
}

/// JSON upload with the file base64-encoded (what the Binary Ninja plugin uses).
#[post("/json")]
async fn push_file_json(
    pool: web::Data<Db>,
    auth: AuthUser,
    body: web::Json<UploadFileJson>,
) -> ApiResult<HttpResponse> {
    let body = body.into_inner();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(body.file.as_bytes())
        .map_err(|_| ApiError::bad_request("file is not valid base64"))?;
    push(
        pool.get_ref(),
        &auth,
        body.source,
        &body.name,
        body.description.as_deref(),
        &bytes,
    )
    .await
}
