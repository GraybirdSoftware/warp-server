//! Request bodies. Names/fields mirror the public server's OpenAPI schemas.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::core::{Format, Role};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommitQuery {
    pub limit: Option<i64>,
    pub page: Option<i64>,
    pub source_id: Option<Uuid>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSource {
    pub name: String,
    /// Empty means "just the caller".
    #[serde(default)]
    pub user_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionQuery {
    pub commit_id: Option<i64>,
    /// Keep only functions that carry at least one of these constraint GUIDs.
    pub constraints: Option<Vec<Uuid>>,
    pub format: Option<Format>,
    pub guids: Option<Vec<Uuid>>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
    pub source_id: Option<Uuid>,
    /// Keep only functions whose source carries at least one of these tags.
    pub source_tags: Option<Vec<String>>,
    pub symbol_id: Option<i64>,
    pub target_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceQuery {
    pub exact: Option<bool>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub page: Option<i64>,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PutSource {
    pub name: Option<String>,
    /// Admin only.
    pub tags: Option<Vec<String>>,
    pub user_ids: Option<Vec<i64>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SymbolQuery {
    pub exact: Option<bool>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub page: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetQuery {
    pub arch: Option<String>,
    pub platform: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeQuery {
    pub exact: Option<bool>,
    pub format: Option<Format>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub page: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserQuery {
    pub exact: Option<bool>,
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub page: Option<i64>,
    pub role: Option<Role>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRole {
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeUsername {
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadFileJson {
    pub description: Option<String>,
    /// Base64 (standard alphabet) encoded `.warp` file. Max 64 MiB decoded.
    pub file: String,
    pub name: String,
    pub source: Uuid,
}

/// `GET /search` query string. Arrays may be given as `source_tags[0]=a` or
/// `source_tags=a&source_tags=b`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub kind: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub function_guid: Option<Uuid>,
    pub commit_id: Option<i64>,
    pub source_id: Option<Uuid>,
    pub source_tags: Option<Vec<String>>,
    pub retrieve_data: Option<bool>,
}

/// `POST /functions/data` (batch fetch used by the Binary Ninja plugin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionIds {
    pub ids: Vec<i64>,
}

/// `POST /types/data` (batch fetch used by the Binary Ninja plugin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIds {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}
