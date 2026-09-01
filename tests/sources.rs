mod util;

use serde_json::json;
use util::{json as body, spawn_app};
use warp_server::models::core::Role;

#[tokio::test]
async fn creating_a_source_makes_the_creator_a_member() {
    let app = spawn_app().await;
    let (dev, dev_key) = app.new_user("dev@test.local", "dev", Role::User).await;

    let resp = app
        .post("/api/v1/sources")
        .json(&json!({ "name": "libfoo", "user_ids": [] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let source = app.create_source(&dev_key, "libfoo").await;

    let resp = app
        .get(&format!("/api/v1/sources/{source}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let fetched = body(resp).await;
    assert_eq!(fetched["name"], "libfoo");
    assert_eq!(fetched["id"], source.to_string());

    let resp = app
        .get(&format!("/api/v1/sources/{source}/users"))
        .send()
        .await
        .unwrap();
    let members = body(resp).await;
    assert_eq!(members.as_array().unwrap().len(), 1);
    assert_eq!(members[0]["user_id"], dev);

    let resp = app
        .post("/api/v1/sources/query")
        .json(&json!({ "user_id": dev }))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total_results"], 1);

    let resp = app
        .post("/api/v1/sources/query")
        .json(&json!({ "user_id": app.admin_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total_results"], 0);

    let resp = app
        .post("/api/v1/sources/query")
        .json(&json!({ "name": "FOO" }))
        .send()
        .await
        .unwrap();
    assert_eq!(
        body(resp).await["total_results"],
        1,
        "name search is case-insensitive substring"
    );

    let resp = app
        .get(&format!("/api/v1/sources/{}", uuid::Uuid::new_v4()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn updating_a_source() {
    let app = spawn_app().await;
    let (dev, dev_key) = app.new_user("dev@test.local", "dev", Role::User).await;
    let (other, other_key) = app.new_user("other@test.local", "other", Role::User).await;
    let source = app.create_source(&dev_key, "libfoo").await;
    let path = format!("/api/v1/sources/{source}");

    // Non-members cannot touch it.
    let resp = app
        .post(&path)
        .bearer_auth(&other_key)
        .json(&json!({ "name": "pwned" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    // Members rename it (the upstream spec spells this route with a trailing slash).
    let resp = app
        .post(&format!("{path}/"))
        .bearer_auth(&dev_key)
        .json(&json!({ "name": "libbar" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["name"], "libbar");

    // Members can change membership; unknown users are a 404.
    let resp = app
        .post(&path)
        .bearer_auth(&dev_key)
        .json(&json!({ "user_ids": [dev, 9999] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let resp = app
        .post(&path)
        .bearer_auth(&dev_key)
        .json(&json!({ "user_ids": [dev, other] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp = app.get(&format!("{path}/users")).send().await.unwrap();
    let members = body(resp).await;
    let ids: Vec<i64> = members
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["user_id"].as_i64().unwrap())
        .collect();
    assert_eq!(ids, vec![dev, other]);

    // Only admins set tags.
    let resp = app
        .post(&path)
        .bearer_auth(&dev_key)
        .json(&json!({ "tags": ["official"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .post(&path)
        .bearer_auth(&app.admin_key)
        .json(&json!({ "tags": ["official", "trusted", " "] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = app.get(&format!("{path}/tags")).send().await.unwrap();
    let tags = body(resp).await;
    let tags: Vec<&str> = tags
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["tag"].as_str().unwrap())
        .collect();
    assert_eq!(tags, vec!["official", "trusted"]);
}

#[tokio::test]
async fn deleting_a_source() {
    let app = spawn_app().await;
    let (_, dev_key) = app.new_user("dev@test.local", "dev", Role::User).await;
    let (_, other_key) = app.new_user("other@test.local", "other", Role::User).await;
    let source = app.create_source(&dev_key, "libfoo").await;
    let path = format!("/api/v1/sources/{source}");

    let resp = app
        .delete(&path)
        .bearer_auth(&other_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .delete(&path)
        .bearer_auth(&dev_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app.get(&path).send().await.unwrap();
    assert_eq!(resp.status(), 404);

    let resp = app
        .delete(&path)
        .bearer_auth(&app.admin_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
