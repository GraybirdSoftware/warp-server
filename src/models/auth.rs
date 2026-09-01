use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// API key metadata (never includes the key itself).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKeyInfo {
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub id: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub name: String,
    pub user_id: i64,
}

/// Returned exactly once, when a key is created.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyMdl {
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub id: i64,
    pub key: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub name: String,
    pub user_id: i64,
}

impl ApiKeyMdl {
    pub fn from_info(info: ApiKeyInfo, key: String) -> Self {
        Self {
            created_at: info.created_at,
            expires_at: info.expires_at,
            id: info.id,
            key,
            last_used_at: info.last_used_at,
            name: info.name,
            user_id: info.user_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUrlResponse {
    pub auth_url: String,
}
