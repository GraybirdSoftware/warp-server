mod util;

use serde_json::json;
use util::{json as body, spawn_app};
use warp_server::models::core::Role;

#[tokio::test]
async fn creating_users_requires_an_admin() {
    let app = spawn_app().await;
    let (_, user_key) = app.new_user("user@test.local", "user", Role::User).await;
    let payload = json!({ "email": "alice@test.local", "name": "alice" });

    let resp = app
        .post("/api/v1/users")
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let resp = app
        .post("/api/v1/users")
        .bearer_auth(&user_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .post("/api/v1/users")
        .bearer_auth(&app.admin_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let created = body(resp).await;
    assert_eq!(created["username"], "alice");
    assert_eq!(created["email"], "alice@test.local");
    assert_eq!(created["role"], "User");

    // Same e-mail twice is a conflict.
    let resp = app
        .post("/api/v1/users")
        .bearer_auth(&app.admin_key)
        .json(&payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);

    // Username defaults to the e-mail's local part.
    let resp = app
        .post("/api/v1/users")
        .bearer_auth(&app.admin_key)
        .json(&json!({ "email": "Bob@Test.local" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let created = body(resp).await;
    assert_eq!(created["username"], "bob");
    assert_eq!(created["email"], "bob@test.local");

    let resp = app
        .post("/api/v1/users")
        .bearer_auth(&app.admin_key)
        .json(&json!({ "email": "not-an-email" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn me_and_personal_api_keys() {
    let app = spawn_app().await;
    let (user_id, user_key) = app.new_user("dev@test.local", "dev", Role::User).await;

    let resp = app
        .get("/api/v1/users/me")
        .bearer_auth(&user_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let me = body(resp).await;
    assert_eq!(me["id"], user_id);
    assert_eq!(me["username"], "dev");
    assert_eq!(me["email"], "dev@test.local");

    let resp = app.get("/api/v1/users/me").send().await.unwrap();
    assert_eq!(resp.status(), 401);

    let resp = app
        .post("/api/v1/users/me/keys")
        .bearer_auth(&user_key)
        .json(&json!({ "name": "laptop" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let created = body(resp).await;
    let new_key = created["key"].as_str().unwrap().to_string();
    assert!(new_key.starts_with("warp_"));
    assert_eq!(created["user_id"], user_id);

    // The new key authenticates.
    let resp = app
        .get("/api/v1/users/me")
        .bearer_auth(&new_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Listing never reveals key material.
    let resp = app
        .get("/api/v1/users/me/keys")
        .bearer_auth(&user_key)
        .send()
        .await
        .unwrap();
    let keys = body(resp).await;
    let keys = keys.as_array().unwrap();
    assert_eq!(keys.len(), 2);
    assert!(keys.iter().all(|k| k.get("key").is_none()));
    assert!(keys.iter().any(|k| k["name"] == "laptop"));
    // last_used_at is recorded.
    assert!(keys.iter().any(|k| !k["last_used_at"].is_null()));
}

#[tokio::test]
async fn querying_and_fetching_users_is_public() {
    let app = spawn_app().await;
    let (alice, _) = app.new_user("alice@test.local", "alice", Role::User).await;
    app.new_user("bob@test.local", "bob", Role::User).await;

    let resp = app
        .post("/api/v1/users/query")
        .json(&json!({ "name": "ali" }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(page["total_results"], 1);
    assert_eq!(page["items"][0]["username"], "alice");
    assert!(
        page["items"][0].get("email").is_none(),
        "public listing must not expose e-mails"
    );

    let resp = app
        .post("/api/v1/users/query")
        .json(&json!({ "name": "ali", "exact": true }))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total_results"], 0);

    let resp = app
        .post("/api/v1/users/query")
        .json(&json!({ "role": "Admin" }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(page["total_results"], 1);
    assert_eq!(page["items"][0]["username"], "admin");

    let resp = app
        .post("/api/v1/users/query")
        .json(&json!({ "limit": 2, "page": 1 }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(page["total_results"], 3);
    assert_eq!(page["total_pages"], 2);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);

    let resp = app
        .get(&format!("/api/v1/users/{alice}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["username"], "alice");

    let resp = app.get("/api/v1/users/9999").send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn changing_roles_and_usernames() {
    let app = spawn_app().await;
    let (alice, alice_key) = app.new_user("alice@test.local", "alice", Role::User).await;
    let (_, bob_key) = app.new_user("bob@test.local", "bob", Role::User).await;

    let resp = app
        .patch(&format!("/api/v1/users/{alice}/role"))
        .bearer_auth(&alice_key)
        .json(&json!({ "role": "Admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .patch(&format!("/api/v1/users/{alice}/role"))
        .bearer_auth(&app.admin_key)
        .json(&json!({ "role": "Admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["role"], "Admin");

    let resp = app
        .patch("/api/v1/users/9999/role")
        .bearer_auth(&app.admin_key)
        .json(&json!({ "role": "User" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // Users rename themselves; others may not.
    let resp = app
        .patch(&format!("/api/v1/users/{alice}/username"))
        .bearer_auth(&bob_key)
        .json(&json!({ "username": "mallory" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .patch(&format!("/api/v1/users/{alice}/username"))
        .bearer_auth(&alice_key)
        .json(&json!({ "username": "alice.w" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["username"], "alice.w");

    let resp = app
        .patch(&format!("/api/v1/users/{alice}/username"))
        .bearer_auth(&alice_key)
        .json(&json!({ "username": "bob" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);

    let resp = app
        .patch(&format!("/api/v1/users/{alice}/username"))
        .bearer_auth(&alice_key)
        .json(&json!({ "username": "not ok!" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn deleting_a_user_revokes_their_keys() {
    let app = spawn_app().await;
    let (alice, alice_key) = app.new_user("alice@test.local", "alice", Role::User).await;
    let (_, bob_key) = app.new_user("bob@test.local", "bob", Role::User).await;

    let resp = app
        .delete(&format!("/api/v1/users/{alice}"))
        .bearer_auth(&bob_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .delete(&format!("/api/v1/users/{alice}"))
        .bearer_auth(&app.admin_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let resp = app
        .get(&format!("/api/v1/users/{alice}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let resp = app
        .get("/api/v1/users/me")
        .bearer_auth(&alice_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let resp = app
        .delete(&format!("/api/v1/users/{alice}"))
        .bearer_auth(&app.admin_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn admins_manage_other_users_keys() {
    let app = spawn_app().await;
    let (alice, _) = app.new_user("alice@test.local", "alice", Role::User).await;
    let (_, bob_key) = app.new_user("bob@test.local", "bob", Role::User).await;

    let resp = app
        .get(&format!("/api/v1/users/{alice}/keys"))
        .bearer_auth(&bob_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    let resp = app
        .post(&format!("/api/v1/users/{alice}/keys"))
        .bearer_auth(&app.admin_key)
        .json(&json!({ "name": "issued-by-admin" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let key = body(resp).await;
    assert_eq!(key["user_id"], alice);

    let resp = app
        .get(&format!("/api/v1/users/{alice}/keys"))
        .bearer_auth(&app.admin_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await.as_array().unwrap().len(), 2);

    let resp = app
        .post("/api/v1/users/9999/keys")
        .bearer_auth(&app.admin_key)
        .json(&json!({ "name": "x" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
