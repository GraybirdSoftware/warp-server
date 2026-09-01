use sqlx::{Executor, Sqlite, SqliteConnection};
use validator::ValidateEmail;

use crate::{
    error::{ApiError, ApiResult},
    models::core::{Role, UserInfo, UserMdl},
};

pub const MDL_COLUMNS: &str = "id, COALESCE(username, email) AS username, email, role, created_at";
pub const INFO_COLUMNS: &str = "id, COALESCE(username, email) AS username, role, created_at";

/// What to do when the requested username is already taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsernamePolicy {
    /// Fail with 409.
    Strict,
    /// Append a numeric suffix (used for OAuth sign-ups).
    Deduplicate,
}

pub async fn count<'e, E>(exec: E) -> ApiResult<i64>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(exec)
        .await
        .map_err(Into::into)
}

pub async fn get_mdl<'e, E>(exec: E, id: i64) -> ApiResult<Option<UserMdl>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, UserMdl>(
        "SELECT id, COALESCE(username, email) AS username, email, role, created_at
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(exec)
    .await
    .map_err(Into::into)
}

pub async fn get_info<'e, E>(exec: E, id: i64) -> ApiResult<Option<UserInfo>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, UserInfo>(
        "SELECT id, COALESCE(username, email) AS username, role, created_at
         FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(exec)
    .await
    .map_err(Into::into)
}

pub async fn find_by_email<'e, E>(exec: E, email: &str) -> ApiResult<Option<UserMdl>>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_as::<_, UserMdl>(
        "SELECT id, COALESCE(username, email) AS username, email, role, created_at
         FROM users WHERE email = ?",
    )
    .bind(normalise_email(email))
    .fetch_optional(exec)
    .await
    .map_err(Into::into)
}

pub async fn username_taken<'e, E>(
    exec: E,
    username: &str,
    exclude_id: Option<i64>,
) -> ApiResult<bool>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE username = ? AND id != ?")
            .bind(username)
            .bind(exclude_id.unwrap_or(-1))
            .fetch_one(exec)
            .await?
            > 0,
    )
}

pub fn normalise_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

/// Keep usernames simple: letters, digits, `.`, `-`, `_`; max 64 chars.
pub fn sanitise_username(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(64)
        .collect();
    if cleaned.is_empty() {
        "user".to_string()
    } else {
        cleaned
    }
}

pub fn validate_username(username: &str) -> ApiResult<()> {
    let trimmed = username.trim();
    if trimmed.is_empty() || trimmed.len() > 64 {
        return Err(ApiError::bad_request(
            "username must be between 1 and 64 characters",
        ));
    }
    if sanitise_username(trimmed) != trimmed {
        return Err(ApiError::bad_request(
            "username may only contain letters, digits, '.', '-' and '_'",
        ));
    }
    Ok(())
}

/// Create a user. `username` defaults to the local part of the e-mail.
pub async fn create(
    conn: &mut SqliteConnection,
    email: &str,
    username: Option<&str>,
    role: Role,
    policy: UsernamePolicy,
) -> ApiResult<UserMdl> {
    let email = normalise_email(email);
    if !email.validate_email() {
        return Err(ApiError::bad_request("invalid email address"));
    }
    if find_by_email(&mut *conn, &email).await?.is_some() {
        return Err(ApiError::Conflict(
            "a user with this email already exists".into(),
        ));
    }

    let requested = username.map(str::trim).filter(|s| !s.is_empty());
    let base = match requested {
        Some(name) => {
            if policy == UsernamePolicy::Strict {
                validate_username(name)?;
            }
            sanitise_username(name)
        }
        None => sanitise_username(email.split('@').next().unwrap_or("user")),
    };

    let mut candidate = base.clone();
    let mut suffix = 1;
    while username_taken(&mut *conn, &candidate, None).await? {
        if requested.is_some() && policy == UsernamePolicy::Strict {
            return Err(ApiError::Conflict("username already taken".into()));
        }
        suffix += 1;
        candidate = format!("{base}{suffix}");
    }

    sqlx::query_as::<_, UserMdl>(
        "INSERT INTO users (username, email, role, created_at) VALUES (?, ?, ?, ?)
         RETURNING id, username, email, role, created_at",
    )
    .bind(candidate)
    .bind(email)
    .bind(role.as_str())
    .bind(super::now())
    .fetch_one(conn)
    .await
    .map_err(Into::into)
}
