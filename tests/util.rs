//! Shared test harness: a fresh SQLite database and server per test.
#![allow(dead_code)]

use std::{net::TcpListener, path::PathBuf};

use reqwest::{Client, RequestBuilder};
use serde_json::{Value, json};
use uuid::Uuid;
use warp_server::{
    bootstrap,
    config::Config,
    db::{self, Db},
    models::core::Role,
    startup,
};

pub struct TestApp {
    pub address: String,
    pub pool: Db,
    pub admin_id: i64,
    pub admin_key: String,
    pub client: Client,
    db_path: PathBuf,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.address, path)
    }

    pub fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(self.url(path))
    }

    pub fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(self.url(path))
    }

    pub fn patch(&self, path: &str) -> RequestBuilder {
        self.client.patch(self.url(path))
    }

    pub fn delete(&self, path: &str) -> RequestBuilder {
        self.client.delete(self.url(path))
    }

    /// Create a user directly in the database and return `(id, api_key)`.
    pub async fn new_user(&self, email: &str, username: &str, role: Role) -> (i64, String) {
        let (user, key) =
            bootstrap::create_user_with_key(&self.pool, email, Some(username), role, "test")
                .await
                .expect("create user");
        (user.id, key.key)
    }

    /// Create a source through the API as the holder of `key`.
    pub async fn create_source(&self, key: &str, name: &str) -> Uuid {
        let resp = self
            .post("/api/v1/sources")
            .bearer_auth(key)
            .json(&json!({ "name": name, "user_ids": [] }))
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            200,
            "create source: {}",
            resp.text().await.unwrap()
        );
        let body: Value = resp.json().await.unwrap();
        Uuid::parse_str(body["id"].as_str().expect("source id")).unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let mut path = self.db_path.clone().into_os_string();
            path.push(suffix);
            let _ = std::fs::remove_file(path);
        }
    }
}

pub async fn spawn_app() -> TestApp {
    warp_server::telemetry::init("warn");
    let dir = std::env::temp_dir().join("warp-server-tests");
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let db_path = dir.join(format!("{}.db", Uuid::new_v4()));
    let url = format!("sqlite://{}?mode=rwc", db_path.display());

    let pool = db::connect_and_migrate(&url)
        .await
        .expect("open test database");
    let (admin, key) = bootstrap::create_user_with_key(
        &pool,
        "admin@test.local",
        Some("admin"),
        Role::Admin,
        "test",
    )
    .await
    .expect("create admin");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind random port");
    let port = listener.local_addr().unwrap().port();
    let server = startup::run(listener, pool.clone(), Config::local()).expect("start server");
    tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{port}"),
        pool,
        admin_id: admin.id,
        admin_key: key.key,
        client: Client::new(),
        db_path,
    }
}

/// Parse a response body as JSON, panicking with the body on failure.
pub async fn json(resp: reqwest::Response) -> Value {
    let status = resp.status();
    let text = resp.text().await.unwrap();
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("invalid JSON ({status}): {e}: {text}"))
}
