//! Turning an uploaded `.warp` file into rows.

use sqlx::SqliteConnection;
use uuid::Uuid;
use warp::{WarpFile, chunk::ChunkKind, signature::function::Function, r#type::ComputedType};

use super::{Db, now, uid, upsert_symbol, upsert_target, upsert_type};
use crate::error::{ApiError, ApiResult};

#[derive(Debug, Default, Clone)]
pub struct IngestSummary {
    pub commit_id: i64,
    pub chunks: usize,
    pub functions: u64,
    pub types: u64,
}

/// Store a WARP file under a new commit of `source_id`.
///
/// Everything happens in one transaction: either the whole file lands or
/// nothing does. Symbols, targets and types are de-duplicated globally;
/// functions are always inserted (the same GUID may legitimately appear in
/// several sources/commits).
pub async fn ingest_warp_file(
    pool: &Db,
    source_id: Uuid,
    user_id: i64,
    name: &str,
    description: Option<&str>,
    bytes: &[u8],
) -> ApiResult<IngestSummary> {
    let file = WarpFile::from_bytes(bytes)
        .ok_or_else(|| ApiError::bad_request("file is not a valid WARP file"))?;

    let source = uid(&source_id);
    let created_at = now();
    let mut tx = pool.begin().await?;

    let commit_id: i64 = sqlx::query_scalar(
        "INSERT INTO commits (created_at, description, name, source_id, user_id)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&created_at)
    .bind(description)
    .bind(name)
    .bind(&source)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO files (commit_id, source_id, name, file, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(commit_id)
    .bind(&source)
    .bind(name)
    .bind(bytes)
    .bind(&created_at)
    .execute(&mut *tx)
    .await?;

    let mut summary = IngestSummary {
        commit_id,
        chunks: file.chunks.len(),
        ..Default::default()
    };

    for chunk in &file.chunks {
        let target_id = upsert_target(&mut *tx, &chunk.header.target).await?;
        match &chunk.kind {
            ChunkKind::Signature(chunk) => {
                let functions: Vec<Function> = chunk.functions().collect();
                for function in &functions {
                    insert_function(
                        &mut tx,
                        function,
                        &source,
                        commit_id,
                        target_id,
                        &created_at,
                    )
                    .await?;
                }
                summary.functions += functions.len() as u64;
            }
            ChunkKind::Type(chunk) => {
                let types: Vec<ComputedType> = chunk.types().collect();
                for computed in &types {
                    if upsert_type(
                        &mut *tx,
                        computed.guid.to_string(),
                        computed.ty.name.clone(),
                        &source,
                        commit_id,
                        computed.ty.to_bytes(),
                        &created_at,
                    )
                    .await?
                    {
                        summary.types += 1;
                    }
                }
            }
        }
    }

    tx.commit().await?;
    Ok(summary)
}

async fn insert_function(
    conn: &mut SqliteConnection,
    function: &Function,
    source: &str,
    commit_id: i64,
    target_id: i64,
    created_at: &str,
) -> ApiResult<i64> {
    let symbol_id = upsert_symbol(&mut *conn, &function.symbol).await?;

    let type_id = match &function.ty {
        Some(ty) => {
            let computed = ComputedType::new(ty.clone());
            let guid = computed.guid.to_string();
            upsert_type(
                &mut *conn,
                guid.clone(),
                ty.name.clone(),
                source,
                commit_id,
                ty.to_bytes(),
                created_at,
            )
            .await?;
            Some(guid)
        }
        None => None,
    };

    let function_id: i64 = sqlx::query_scalar(
        "INSERT INTO functions (guid, commit_id, source_id, target_id, symbol_id, type_id, data, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(function.guid.to_string())
    .bind(commit_id)
    .bind(source.to_string())
    .bind(target_id)
    .bind(symbol_id)
    .bind(type_id)
    .bind(function.to_bytes())
    .bind(created_at.to_string())
    .fetch_one(&mut *conn)
    .await?;

    for constraint in &function.constraints {
        sqlx::query(
            "INSERT INTO function_constraints (function_id, guid, byte_offset) VALUES (?, ?, ?)",
        )
        .bind(function_id)
        .bind(constraint.guid.to_string())
        .bind(constraint.offset)
        .execute(&mut *conn)
        .await?;
    }

    for comment in &function.comments {
        sqlx::query(
            "INSERT INTO function_comments (function_id, text, byte_offset) VALUES (?, ?, ?)",
        )
        .bind(function_id)
        .bind(comment.text.clone())
        .bind(comment.offset)
        .execute(&mut *conn)
        .await?;
    }

    Ok(function_id)
}
