//! Manual cascading deletes (the schema does not rely on SQLite foreign keys).

use sqlx::SqliteConnection;

use crate::error::ApiResult;

pub async fn delete_commit(conn: &mut SqliteConnection, commit_id: i64) -> ApiResult<()> {
    sqlx::query(
        "DELETE FROM function_constraints
         WHERE function_id IN (SELECT id FROM functions WHERE commit_id = ?)",
    )
    .bind(commit_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "DELETE FROM function_comments
         WHERE function_id IN (SELECT id FROM functions WHERE commit_id = ?)",
    )
    .bind(commit_id)
    .execute(&mut *conn)
    .await?;
    for table in ["functions", "types", "files"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE commit_id = ?"))
            .bind(commit_id)
            .execute(&mut *conn)
            .await?;
    }
    sqlx::query("DELETE FROM commits WHERE id = ?")
        .bind(commit_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

pub async fn delete_source(conn: &mut SqliteConnection, source_id: &str) -> ApiResult<()> {
    let commits: Vec<i64> = sqlx::query_scalar("SELECT id FROM commits WHERE source_id = ?")
        .bind(source_id)
        .fetch_all(&mut *conn)
        .await?;
    for commit_id in commits {
        delete_commit(&mut *conn, commit_id).await?;
    }
    for table in ["source_users", "source_tags"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE source_id = ?"))
            .bind(source_id)
            .execute(&mut *conn)
            .await?;
    }
    sqlx::query("DELETE FROM source WHERE id = ?")
        .bind(source_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Remove a user. Their commits are kept (authorship becomes anonymous).
pub async fn delete_user(conn: &mut SqliteConnection, user_id: i64) -> ApiResult<()> {
    for table in ["api_keys", "sessions", "source_users"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE user_id = ?"))
            .bind(user_id)
            .execute(&mut *conn)
            .await?;
    }
    sqlx::query("UPDATE commits SET user_id = NULL WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}
