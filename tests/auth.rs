mod util;

use serde_json::json;
use util::{json as body, spawn_app};
use warp_server::models::core::Role;

#[tokio::test]
async fn malformed_and_unknown_keys_are_rejected() {
    let app = spawn_app().await;

    let resp = app
        .get("/api/v1/users/me")
        .bearer_auth("warp_not_a_real_key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    assert_eq!(resp.headers().get("www-authenticate").unwrap(), "Bearer");

    let resp = app
        .get("/api/v1/users/me")
        .header("Authorization", "Basic abc")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn expired_keys_are_rejected() {
    let app = spawn_app().await;
    let (user_id, key) = app.new_user("dev@test.local", "dev", Role::User).await;

    let resp = app
        .get("/api/v1/users/me")
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    sqlx::query("UPDATE api_keys SET expires_at = ? WHERE user_id = ?")
        .bind("2000-01-01T00:00:00+00:00")
        .bind(user_id)
        .execute(&app.pool)
        .await
        .unwrap();

    let resp = app
        .get("/api/v1/users/me")
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn oauth_login_is_unavailable_without_configuration() {
    let app = spawn_app().await;

    let resp = app
        .post("/api/v1/auth/login")
        .json(&json!({ "next": "/" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);
    assert!(
        body(resp).await["error"]
            .as_str()
            .unwrap()
            .contains("OAuth")
    );

    let resp = app
        .get("/api/v1/auth/o/callback?code=x&state=y")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);

    // Logging out is always fine.
    let resp = app.post("/api/v1/auth/logout").send().await.unwrap();
    assert_eq!(resp.status(), 204);
}
