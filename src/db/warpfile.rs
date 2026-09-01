//! Re-assembling stored functions/types into WARP files for clients.

use std::collections::BTreeMap;

use actix_web::HttpResponse;
use sqlx::FromRow;
use uuid::Uuid;
use warp::{
    WarpFile, WarpFileHeader,
    chunk::{Chunk, ChunkKind, CompressionType},
    signature::{chunk::SignatureChunk, function::Function},
    target::Target,
    r#type::{ComputedType, Type, chunk::TypeChunk, guid::TypeGUID},
};

/// Columns needed to rebuild a function inside a target-specific chunk.
pub const FUNCTION_BLOB_SELECT: &str =
    "SELECT f.id, f.data, t.platform, t.arch FROM functions f JOIN targets t ON t.id = f.target_id";

#[derive(Debug, FromRow)]
pub struct FunctionBlob {
    pub id: i64,
    pub data: Vec<u8>,
    pub platform: String,
    pub arch: String,
}

#[derive(Debug, FromRow)]
pub struct TypeBlob {
    pub id: String,
    pub data: Vec<u8>,
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// Build a WARP file with one signature chunk per target.
pub fn function_file(rows: Vec<FunctionBlob>) -> Vec<u8> {
    let mut by_target: BTreeMap<(String, String), Vec<Function>> = BTreeMap::new();
    for row in rows {
        match Function::from_bytes(&row.data) {
            Some(function) => by_target
                .entry((row.platform, row.arch))
                .or_default()
                .push(function),
            None => tracing::warn!(
                function_id = row.id,
                "stored function is not a valid WARP function; skipped"
            ),
        }
    }

    let mut chunks = Vec::with_capacity(by_target.len());
    for ((platform, arch), functions) in by_target {
        let Some(chunk) = SignatureChunk::new(&functions) else {
            tracing::warn!(%platform, %arch, "failed to build signature chunk; skipped");
            continue;
        };
        let target = Target {
            architecture: non_empty(arch),
            platform: non_empty(platform),
        };
        chunks.push(Chunk::new_with_target(
            ChunkKind::Signature(chunk),
            CompressionType::Zstd,
            target,
        ));
    }
    WarpFile::new(WarpFileHeader::new(), chunks).to_bytes()
}

/// Build a WARP file containing a single type chunk.
pub fn type_file(rows: Vec<TypeBlob>) -> Vec<u8> {
    let mut types = Vec::with_capacity(rows.len());
    for row in rows {
        let Ok(uuid) = Uuid::parse_str(&row.id) else {
            tracing::warn!(type_id = %row.id, "type id is not a UUID; skipped");
            continue;
        };
        match Type::from_bytes(&row.data) {
            Some(ty) => types.push(ComputedType::new_with_guid(ty, TypeGUID::from(uuid))),
            None => {
                tracing::warn!(type_id = %row.id, "stored type is not a valid WARP type; skipped")
            }
        }
    }

    let mut chunks = Vec::new();
    if !types.is_empty() {
        match TypeChunk::new_with_computed(&types) {
            Some(chunk) => chunks.push(Chunk::new(ChunkKind::Type(chunk), CompressionType::Zstd)),
            None => tracing::warn!("failed to build type chunk"),
        }
    }
    WarpFile::new(WarpFileHeader::new(), chunks).to_bytes()
}

pub fn octet_stream(bytes: Vec<u8>) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(bytes)
}
