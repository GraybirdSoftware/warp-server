//! Row/response models. Field names match the public server's OpenAPI schemas.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::fmt::Hyphenated;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "User",
            Role::Admin => "Admin",
        }
    }
}

/// Response format for routes that can answer with WARP flatbuffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Json,
    Flatbuffer,
}

/// Full user record (only shown to the user themselves and admins).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserMdl {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub email: String,
    pub role: Role,
    pub username: String,
}

/// Public user record.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SourceMdl {
    pub id: Hyphenated,
    pub created_at: DateTime<Utc>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SourceTagMdl {
    pub source_id: Hyphenated,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SourceUserMdl {
    pub source_id: Hyphenated,
    pub user_id: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct CommitMdl {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub source_id: Hyphenated,
    pub user_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TargetMdl {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub platform: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SymbolMdl {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub name: String,
    pub class: String,
    pub modifier: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TypeMdl {
    pub id: Hyphenated,
    pub created_at: DateTime<Utc>,
    pub commit_id: i64,
    pub source_id: Hyphenated,
    pub name: Option<String>,
    /// Raw WARP `Type` flatbuffer bytes.
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FunctionMdl {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub commit_id: i64,
    pub target_id: i64,
    pub source_id: Hyphenated,
    pub guid: Hyphenated,
    pub symbol_id: i64,
    pub type_id: Option<Hyphenated>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FunctionConstraintMdl {
    pub id: i64,
    pub function_id: i64,
    pub guid: Hyphenated,
    pub byte_offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FunctionCommentMdl {
    pub id: i64,
    pub function_id: i64,
    pub text: String,
    pub byte_offset: i64,
}
