//! The Binary Ninja client labels every request `Content-Encoding: gzip`,
//! compressed or not. Both shapes must work.
mod util;

use std::io::Write;

use flate2::{Compression, write::GzEncoder};
use serde_json::json;
use util::{json as body, spawn_app};
use warp_server::models::core::Role;

#[tokio::test]
async fn plain_body_with_gzip_header_is_accepted() {
    let app = spawn_app().await;
    let (_, key) = app.new_user("dev@test.local", "dev", Role::User).await;

    let resp = app
        .post("/api/v1/sources/query")
        .header("Content-Encoding", "gzip")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["total_results"], 0);

    // Exactly what the plugin sends when creating a source.
    let resp = app
        .post("/api/v1/sources")
        .bearer_auth(&key)
        .header("Content-Encoding", "gzip")
        .json(&json!({ "name": "from-binja", "user_ids": [] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["name"], "from-binja");
}

#[tokio::test]
async fn gzipped_body_is_decoded() {
    let app = spawn_app().await;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(br#"{"name":"nothing-matches"}"#).unwrap();
    let compressed = encoder.finish().unwrap();

    let resp = app
        .post("/api/v1/sources/query")
        .header("Content-Encoding", "gzip")
        .header("Content-Type", "application/json")
        .body(compressed)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["total_results"], 0);
}

#[tokio::test]
async fn corrupt_gzip_is_a_bad_request() {
    let app = spawn_app().await;

    let resp = app
        .post("/api/v1/sources/query")
        .header("Content-Encoding", "gzip")
        .header("Content-Type", "application/json")
        .body(b"\x1f\x8bthis is not gzip".to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn requests_without_the_header_are_untouched() {
    let app = spawn_app().await;

    let resp = app
        .post("/api/v1/sources/query")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp = app
        .get("/api/v1/status")
        .header("Content-Encoding", "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
