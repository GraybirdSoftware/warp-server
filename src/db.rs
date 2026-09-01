//! Database helpers shared by the handlers.

pub mod api_keys;
pub mod cascade;
pub mod ingest;
pub mod users;
pub mod warpfile;

use std::{str::FromStr, time::Duration};

use sqlx::{
    Executor, Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use uuid::Uuid;
use warp::{
    symbol::{Symbol, SymbolClass, SymbolModifiers},
    target::Target,
};

use crate::error::ApiResult;

pub type Db = Pool<Sqlite>;

/// Open (creating if needed) the SQLite database and apply migrations.
pub async fn connect_and_migrate(
    url: &str,
) -> Result<Db, Box<dyn std::error::Error + Send + Sync>> {
    let options = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(10));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// RFC 3339 timestamp for `created_at`-style columns.
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Canonical textual form used for every UUID column.
pub fn uid(id: &Uuid) -> String {
    id.hyphenated().to_string()
}

/// `%term%` with LIKE wildcards escaped; pair with `LIKE ? ESCAPE '\'`.
pub fn like_contains(term: &str) -> String {
    let mut out = String::with_capacity(term.len() + 2);
    out.push('%');
    for ch in term.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('%');
    out
}

/// JSON array of strings, for `IN (SELECT value FROM json_each(?))` filters.
/// This sidesteps SQLite's bound-parameter limit for long GUID lists.
pub fn json_strings<I, S>(items: I) -> String
where
    I: IntoIterator<Item = S>,
    S: ToString,
{
    let items: Vec<String> = items.into_iter().map(|s| s.to_string()).collect();
    serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
}

pub fn json_ints(items: &[i64]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

pub async fn source_exists<'e, E>(exec: E, source_id: &str) -> ApiResult<bool>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM source WHERE id = ?")
            .bind(source_id)
            .fetch_one(exec)
            .await?
            > 0,
    )
}

pub async fn is_source_member<'e, E>(exec: E, source_id: &str, user_id: i64) -> ApiResult<bool>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM source_users WHERE source_id = ? AND user_id = ?",
    )
    .bind(source_id)
    .bind(user_id)
    .fetch_one(exec)
    .await?
        > 0)
}

pub async fn user_exists<'e, E>(exec: E, user_id: i64) -> ApiResult<bool>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(exec)
            .await?
            > 0,
    )
}

pub fn symbol_class_name(class: SymbolClass) -> &'static str {
    if class == SymbolClass::Function {
        "Function"
    } else if class == SymbolClass::Data {
        "Data"
    } else {
        "Bare"
    }
}

pub fn symbol_modifier_name(modifiers: SymbolModifiers) -> &'static str {
    if modifiers.contains(SymbolModifiers::Exported) {
        "Exported"
    } else if modifiers.contains(SymbolModifiers::External) {
        "Extern"
    } else {
        "None"
    }
}

/// Find-or-create the target row for a chunk's target, returning its id.
pub async fn upsert_target<'e, E>(exec: E, target: &Target) -> ApiResult<i64>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO targets (platform, arch, created_at) VALUES (?, ?, ?)
         ON CONFLICT(platform, arch) DO UPDATE SET platform = excluded.platform
         RETURNING id",
    )
    .bind(target.platform.clone().unwrap_or_default())
    .bind(target.architecture.clone().unwrap_or_default())
    .bind(now())
    .fetch_one(exec)
    .await
    .map_err(Into::into)
}

/// Find-or-create the symbol row, returning its id.
pub async fn upsert_symbol<'e, E>(exec: E, symbol: &Symbol) -> ApiResult<i64>
where
    E: Executor<'e, Database = Sqlite>,
{
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO symbols (name, class, modifier, created_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(name, class, modifier) DO UPDATE SET name = excluded.name
         RETURNING id",
    )
    .bind(symbol.name.clone())
    .bind(symbol_class_name(symbol.class))
    .bind(symbol_modifier_name(symbol.modifiers))
    .bind(now())
    .fetch_one(exec)
    .await
    .map_err(Into::into)
}

/// Insert a type keyed by its GUID; an existing GUID is left untouched.
pub async fn upsert_type<'e, E>(
    exec: E,
    guid: String,
    name: Option<String>,
    source_id: &str,
    commit_id: i64,
    data: Vec<u8>,
    created_at: &str,
) -> ApiResult<bool>
where
    E: Executor<'e, Database = Sqlite>,
{
    let result = sqlx::query(
        "INSERT INTO types (id, name, source_id, commit_id, data, created_at)
         VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING",
    )
    .bind(guid)
    .bind(name)
    .bind(source_id.to_string())
    .bind(commit_id)
    .bind(data)
    .bind(created_at.to_string())
    .execute(exec)
    .await?;
    Ok(result.rows_affected() > 0)
}
