mod util;

use util::{json, spawn_app};

#[tokio::test]
async fn status_reports_ok() {
    let app = spawn_app().await;

    let resp = app.get("/api/v1/status").send().await.unwrap();
    assert!(resp.status().is_success());
    let body = json(resp).await;
    assert_eq!(body["status"], "OK");
}

#[tokio::test]
async fn trailing_slashes_are_tolerated() {
    let app = spawn_app().await;

    let resp = app.get("/api/v1/status/").send().await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn unknown_routes_are_404() {
    let app = spawn_app().await;

    let resp = app.get("/api/v1/nope").send().await.unwrap();
    assert_eq!(resp.status(), 404);
}
