use sqlx::{Pool, Sqlite};

pub async fn create_pool() -> Pool<Sqlite> {
    let pool = sqlx::sqlite::SqlitePool::connect("sqlite://wos.db?mode=rwc")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}
