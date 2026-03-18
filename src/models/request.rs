use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::core::Format;

#[derive(Serialize, Deserialize)]
pub struct CommitQuery {
    limit: Option<i64>,
    page: Option<i64>,
    source_id: Option<Uuid>,
    user_id: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateApiKey {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSource {
    name: String,
    user_ids: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUser {
    email: String,
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionQuery {
    commit_id: Option<i32>,
    constraints: Option<Vec<String>>,
    format: Option<Format>,
    guids: Option<Vec<String>>,
    limit: Option<i64>,
    page: Option<i64>,
    source_id: Option<Uuid>,
    source_tags: Option<Vec<String>>,
    symbol_id: Option<i32>,
    target_id: Option<i32>,
}
