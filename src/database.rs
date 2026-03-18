use sqlx::{Pool, Sqlite};
use tokio::sync::OnceCell;

pub async fn create_pool() -> Pool<Sqlite> {
    use sqlx::SqlitePool;

    let pool = sqlx::sqlite::SqlitePool::connect("sqlite://wos.db?mode=rwc")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

