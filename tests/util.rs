use std::net::TcpListener;
use sqlx::{Pool, Sqlite};
pub struct AppConfig { 
    pub address: String,
    pub pool: Pool<Sqlite>
}

pub async fn start_test_instance() -> AppConfig { 
    let pool = sqlx::sqlite::SqlitePool::connect("sqlite://sqlite_test.db?mode=rwc")
        .await
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let server = warp_server::startup::run(listener, pool.clone()).unwrap();

    let _ = tokio::spawn(server);

    AppConfig { 
        address,
        pool
    }

}