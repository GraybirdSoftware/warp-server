use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::fmt::Hyphenated;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResponse {
    pub commit_id: i64,
}

/// One hit from `GET /search`.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct SearchIndexRow {
    /// `source` | `function` | `type` | `symbol`
    pub kind: String,
    pub id: String,
    pub name: Option<String>,
    pub source_id: Option<Hyphenated>,
    pub commit_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub function_guid: Option<Hyphenated>,
    pub symbol_id: Option<i64>,
    /// WARP function/type bytes when `retrieve_data=true`.
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub items: Vec<SearchIndexRow>,
    pub limit: i64,
    pub offset: i64,
    pub total: i64,
}
