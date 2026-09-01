use sqlx::{Executor, Sqlite};

use crate::{
    auth,
    error::{ApiError, ApiResult},
    models::auth::{ApiKeyInfo, ApiKeyMdl},
};

const INFO_COLUMNS: &str = "id, user_id, name, created_at, expires_at, last_used_at";

/// Mint a key for `user_id`. The plaintext key is only available in the
/// returned value; the database keeps its SHA-256 digest.
pub async fn create<'e, E>(exec: E, user_id: i64, name: &str) -> ApiResult<ApiKeyMdl>
where
    E: Executor<'e, Database = Sqlite>,
{
    let name = name.trim();
    if name.is_empty() || name.len() > 128 {
        return Err(ApiError::bad_request(
            "key name must be between 1 and 128 characters",
        ));
    }
    let token = auth::generate_token();
    let info = sqlx::query_as::<_, ApiKeyInfo>(
        "INSERT INTO api_keys (user_id, name, key, created_at) VALUES (?, ?, ?, ?)
         RETURNING id, user_id, name, created_at, expires_at, last_used_at",
    )
    .bind(user_id)
    .bind(name)
    .bind(auth::hash_token(&token))
    .bind(super::now())
    .fetch_one(exec)
    .await?;
    Ok(ApiKeyMdl::from_info(info, token))
}

pub async fn list<'e, E>(exec: E, user_id: i64) -> ApiResult<Vec<ApiKeyInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, ApiKeyInfo>(&format!(
        "SELECT {INFO_COLUMNS} FROM api_keys WHERE user_id = ? ORDER BY id"
    ))
    .bind(user_id)
    .fetch_all(exec)
    .await
    .map_err(Into::into)
}
