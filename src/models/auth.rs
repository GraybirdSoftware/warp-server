use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ApiKeyInfo {
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    id: i32,
    last_used_at: Option<DateTime<Utc>>,
    name: String,
    user_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ApiKeyMdl {
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    id: i32,
    key: String,
    last_used_at: Option<DateTime<Utc>>,
    name: String,
    user_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct AuthUrlResponse {
    auth_url: String,
}
