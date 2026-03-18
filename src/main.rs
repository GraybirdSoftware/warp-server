use std::net::TcpListener;


#[tokio::main]
async fn main() -> std::io::Result<()> {

    let pool = sqlx::sqlite::SqlitePool::connect("sqlite://sqlite.db?mode=rwc")
        .await
        .unwrap();

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let port = 80;
    let address = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(address)?;

    warp_server::startup::run(listener, pool)?.await?;

    Ok(())
    
}
