//! End-to-end: push a WARP file, then read it back through every route.
mod util;

use base64::Engine;
use serde_json::{Value, json};
use util::{TestApp, json as body, spawn_app};
use uuid::Uuid;
use warp::{
    WarpFile, WarpFileHeader,
    chunk::{Chunk, ChunkKind, CompressionType},
    mock::{
        mock_constraint, mock_constraint_guid, mock_function, mock_function_guid,
        mock_int_type_class, mock_struct_type_class, mock_type,
    },
    signature::{chunk::SignatureChunk, comment::FunctionComment, function::Function},
    target::Target,
    r#type::{Type, chunk::TypeChunk, guid::TypeGUID},
};
use warp_server::models::core::Role;

fn my_struct() -> Type {
    let int = mock_type("int32_t", mock_int_type_class(Some(32), true));
    mock_type(
        "my_struct",
        mock_struct_type_class(&[(0, "a", &int), (32, "b", &int)]),
    )
}

/// Two signature chunks (x86_64 and aarch64) plus one type chunk.
fn sample_file() -> WarpFile<'static> {
    let mut alpha = mock_function("alpha");
    alpha
        .constraints
        .insert(mock_constraint("calls_beta", Some(0x10)));
    alpha.comments.push(FunctionComment {
        offset: 4,
        text: "hello".to_string(),
    });
    let beta = mock_function("beta");
    let gamma = mock_function("gamma");

    let x86 = Target {
        architecture: Some("x86_64".into()),
        platform: Some("linux".into()),
    };
    let arm = Target {
        architecture: Some("aarch64".into()),
        platform: Some("linux".into()),
    };

    let chunks = vec![
        Chunk::new_with_target(
            ChunkKind::Signature(SignatureChunk::new(&[alpha, beta]).unwrap()),
            CompressionType::Zstd,
            x86,
        ),
        Chunk::new_with_target(
            ChunkKind::Signature(SignatureChunk::new(&[gamma]).unwrap()),
            CompressionType::None,
            arm,
        ),
        Chunk::new(
            ChunkKind::Type(TypeChunk::new(&[my_struct()]).unwrap()),
            CompressionType::None,
        ),
    ];
    WarpFile::new(WarpFileHeader::new(), chunks)
}

fn encode(file: &WarpFile<'_>) -> String {
    base64::engine::general_purpose::STANDARD.encode(file.to_bytes())
}

async fn push(app: &TestApp, key: &str, source: Uuid, file: &WarpFile<'_>) -> reqwest::Response {
    app.post("/api/v1/files/json")
        .bearer_auth(key)
        .json(&json!({ "file": encode(file), "name": "commit", "source": source, "description": null }))
        .send()
        .await
        .unwrap()
}

struct Pushed {
    app: TestApp,
    dev_key: String,
    source: Uuid,
    commit_id: i64,
}

async fn pushed() -> Pushed {
    let app = spawn_app().await;
    let (_, dev_key) = app.new_user("dev@test.local", "dev", Role::User).await;
    let source = app.create_source(&dev_key, "libfoo").await;
    let resp = push(&app, &dev_key, source, &sample_file()).await;
    assert_eq!(resp.status(), 200, "{}", resp.text().await.unwrap());
    let commit_id = body(resp).await["commit_id"].as_i64().unwrap();
    Pushed {
        app,
        dev_key,
        source,
        commit_id,
    }
}

async fn query_functions(app: &TestApp, filter: Value) -> Value {
    let resp = app
        .post("/api/v1/functions/query")
        .json(&filter)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    body(resp).await
}

fn count_functions(file: &WarpFile<'_>) -> usize {
    file.chunks
        .iter()
        .map(|c| match &c.kind {
            ChunkKind::Signature(sc) => sc.functions().count(),
            ChunkKind::Type(_) => 0,
        })
        .sum()
}

fn bytes_of(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_u64().unwrap() as u8)
        .collect()
}

#[tokio::test]
async fn pushed_functions_are_queryable_as_json() {
    let p = pushed().await;
    let alpha = mock_function_guid("alpha").to_string();

    let page = query_functions(&p.app, json!({})).await;
    assert_eq!(page["total_results"], 3);
    assert_eq!(page["total_pages"], 1);

    let page = query_functions(&p.app, json!({ "guids": [alpha] })).await;
    assert_eq!(page["total_results"], 1);
    let item = &page["items"][0];
    assert_eq!(item["guid"], alpha);
    assert_eq!(item["source_id"], p.source.to_string());
    assert_eq!(item["commit_id"], p.commit_id);
    assert!(!item["type_id"].is_null(), "mock functions carry a type");

    let page = query_functions(&p.app, json!({ "guids": [Uuid::new_v4()] })).await;
    assert_eq!(page["total_results"], 0);

    let page = query_functions(&p.app, json!({ "source_id": Uuid::new_v4() })).await;
    assert_eq!(page["total_results"], 0);

    let page = query_functions(&p.app, json!({ "commit_id": p.commit_id, "limit": 2 })).await;
    assert_eq!(page["total_results"], 3);
    assert_eq!(page["total_pages"], 2);
    assert_eq!(page["items"].as_array().unwrap().len(), 2);

    // Filter by target.
    let resp = p
        .app
        .post("/api/v1/targets/query")
        .json(&json!({ "arch": "aarch64" }))
        .send()
        .await
        .unwrap();
    let targets = body(resp).await;
    assert_eq!(targets.as_array().unwrap().len(), 1);
    let arm_id = targets[0]["id"].as_i64().unwrap();
    assert_eq!(targets[0]["platform"], "linux");
    let page = query_functions(&p.app, json!({ "target_id": arm_id })).await;
    assert_eq!(page["total_results"], 1);
    assert_eq!(
        page["items"][0]["guid"],
        mock_function_guid("gamma").to_string()
    );

    // Filter by constraints: any overlap keeps the function.
    let page = query_functions(
        &p.app,
        json!({ "constraints": [mock_constraint_guid("calls_beta").to_string()] }),
    )
    .await;
    assert_eq!(page["total_results"], 1);
    assert_eq!(page["items"][0]["guid"], alpha);
    let page = query_functions(&p.app, json!({ "constraints": [Uuid::new_v4()] })).await;
    assert_eq!(page["total_results"], 0);

    // Filter by source tags (none set yet).
    let page = query_functions(&p.app, json!({ "source_tags": ["official"] })).await;
    assert_eq!(page["total_results"], 0);
    let resp = p
        .app
        .post(&format!("/api/v1/sources/{}", p.source))
        .bearer_auth(&p.app.admin_key)
        .json(&json!({ "tags": ["official"] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let page = query_functions(&p.app, json!({ "source_tags": ["official", "trusted"] })).await;
    assert_eq!(page["total_results"], 3);
}

#[tokio::test]
async fn flatbuffer_responses_round_trip_through_the_warp_crate() {
    let p = pushed().await;

    let resp = p
        .app
        .post("/api/v1/functions/query")
        .json(&json!({ "format": "flatbuffer", "guids": [], "limit": 10000 }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["content-type"], "application/octet-stream");
    let file =
        WarpFile::from_owned_bytes(resp.bytes().await.unwrap().to_vec()).expect("valid WARP file");
    assert_eq!(file.chunks.len(), 2, "one signature chunk per target");
    assert_eq!(count_functions(&file), 3);
    let mut archs: Vec<String> = file
        .chunks
        .iter()
        .filter_map(|c| c.header.target.architecture.clone())
        .collect();
    archs.sort();
    assert_eq!(archs, vec!["aarch64", "x86_64"]);

    // The alpha function keeps its constraints, comments and type.
    let alpha = file
        .chunks
        .iter()
        .find_map(|c| match &c.kind {
            ChunkKind::Signature(sc) => sc.functions_with_guid(&mock_function_guid("alpha")).next(),
            _ => None,
        })
        .expect("alpha present");
    assert_eq!(alpha.constraints.len(), 1);
    assert_eq!(alpha.comments[0].text, "hello");
    assert_eq!(
        alpha.ty.as_ref().and_then(|t| t.name.clone()).as_deref(),
        Some("alpha")
    );

    // Restricting by GUID narrows the file.
    let resp = p
        .app
        .post("/api/v1/functions/query")
        .json(
            &json!({ "format": "flatbuffer", "guids": [mock_function_guid("gamma").to_string()] }),
        )
        .send()
        .await
        .unwrap();
    let file = WarpFile::from_owned_bytes(resp.bytes().await.unwrap().to_vec()).unwrap();
    assert_eq!(count_functions(&file), 1);

    // Batch data endpoint used by the plugin.
    let page = query_functions(&p.app, json!({})).await;
    let ids: Vec<i64> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_i64().unwrap())
        .collect();
    let resp = p
        .app
        .post("/api/v1/functions/data")
        .json(&json!({ "ids": ids }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let file = WarpFile::from_owned_bytes(resp.bytes().await.unwrap().to_vec()).unwrap();
    assert_eq!(count_functions(&file), 3);
}

#[tokio::test]
async fn query_source_maps_sources_to_guids() {
    let p = pushed().await;
    let guids = ["alpha", "beta", "gamma"].map(|m| mock_function_guid(m).to_string());

    let resp = p
        .app
        .post("/api/v1/functions/query/source")
        .json(&json!({ "guids": [guids[0], guids[2], Uuid::new_v4()] }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let mapped = body(resp).await;
    let mapped = mapped.as_object().unwrap();
    assert_eq!(mapped.len(), 1);
    let found = mapped[&p.source.to_string()].as_array().unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.contains(&json!(guids[0])));
    assert!(found.contains(&json!(guids[2])));
    assert!(!found.contains(&json!(guids[1])));
}

#[tokio::test]
async fn function_detail_routes() {
    let p = pushed().await;
    let page = query_functions(
        &p.app,
        json!({ "guids": [mock_function_guid("alpha").to_string()] }),
    )
    .await;
    let id = page["items"][0]["id"].as_i64().unwrap();
    let base = format!("/api/v1/functions/{id}");

    let resp = p.app.get(&base).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(body(resp).await["id"], id);

    let resp = p.app.get(&format!("{base}/symbol")).send().await.unwrap();
    let symbol = body(resp).await;
    assert_eq!(symbol["name"], "alpha");
    assert_eq!(symbol["class"], "Function");
    assert_eq!(symbol["modifier"], "None");

    let resp = p
        .app
        .get(&format!("{base}/constraints"))
        .send()
        .await
        .unwrap();
    let constraints = body(resp).await;
    assert_eq!(constraints.as_array().unwrap().len(), 1);
    assert_eq!(
        constraints[0]["guid"],
        mock_constraint_guid("calls_beta").to_string()
    );
    assert_eq!(constraints[0]["byte_offset"], 0x10);

    let resp = p.app.get(&format!("{base}/comments")).send().await.unwrap();
    let comments = body(resp).await;
    assert_eq!(comments[0]["text"], "hello");
    assert_eq!(comments[0]["byte_offset"], 4);

    let resp = p.app.get(&format!("{base}/type")).send().await.unwrap();
    let ty = body(resp).await;
    assert_eq!(ty["name"], "alpha");
    assert!(!bytes_of(&ty["data"]).is_empty());

    let resp = p.app.get(&format!("{base}/data")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let function = Function::from_bytes(&resp.bytes().await.unwrap()).expect("valid function");
    assert_eq!(function.guid, mock_function_guid("alpha"));
    assert_eq!(function.symbol.name, "alpha");

    for suffix in ["", "/symbol", "/constraints", "/comments", "/type", "/data"] {
        let resp = p
            .app
            .get(&format!("/api/v1/functions/9999{suffix}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404, "{suffix}");
    }
}

#[tokio::test]
async fn targets_symbols_and_types() {
    let p = pushed().await;

    // Targets.
    let resp = p
        .app
        .post("/api/v1/targets/query")
        .json(&json!({ "platform": "linux" }))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await.as_array().unwrap().len(), 2);
    let resp = p
        .app
        .get("/api/v1/targets/query?platform=linux&arch=x86_64")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert!(
        resp.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/plain")
    );
    let id: i64 = resp.text().await.unwrap().parse().unwrap();
    let resp = p
        .app
        .get(&format!("/api/v1/targets/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["arch"], "x86_64");
    let resp = p
        .app
        .get("/api/v1/targets/query?arch=mips")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // Symbols.
    let resp = p
        .app
        .post("/api/v1/symbols/query")
        .json(&json!({ "name": "a" }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(
        page["total_results"], 3,
        "alpha, beta, gamma all contain 'a'"
    );
    let resp = p
        .app
        .post("/api/v1/symbols/query")
        .json(&json!({ "name": "beta", "exact": true }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(page["total_results"], 1);
    let symbol_id = page["items"][0]["id"].as_i64().unwrap();
    let resp = p
        .app
        .get(&format!("/api/v1/symbols/{symbol_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["name"], "beta");
    let page = query_functions(&p.app, json!({ "symbol_id": symbol_id })).await;
    assert_eq!(page["total_results"], 1);

    // Types: one from the type chunk plus one per distinct function type.
    let struct_guid = TypeGUID::from(&my_struct()).to_string();
    let resp = p
        .app
        .post("/api/v1/types/query")
        .json(&json!({ "name": "my_struct" }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(page["total_results"], 1);
    assert_eq!(page["items"][0]["id"], struct_guid);
    assert_eq!(page["items"][0]["commit_id"], p.commit_id);

    let resp = p
        .app
        .get(&format!("/api/v1/types/{struct_guid}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let resp = p
        .app
        .get(&format!("/api/v1/types/{struct_guid}/data"))
        .send()
        .await
        .unwrap();
    let ty = Type::from_bytes(&resp.bytes().await.unwrap()).expect("valid type");
    assert_eq!(ty.name.as_deref(), Some("my_struct"));
    assert_eq!(ty, my_struct());

    let resp = p
        .app
        .post("/api/v1/types/query")
        .json(&json!({ "format": "flatbuffer" }))
        .send()
        .await
        .unwrap();
    let file = WarpFile::from_owned_bytes(resp.bytes().await.unwrap().to_vec()).unwrap();
    let types: Vec<_> = file
        .chunks
        .iter()
        .flat_map(|c| match &c.kind {
            ChunkKind::Type(tc) => tc.types().collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect();
    assert_eq!(
        types.len(),
        4,
        "my_struct + alpha/beta/gamma function types"
    );

    let resp = p
        .app
        .post("/api/v1/types/data")
        .json(&json!({ "ids": [struct_guid] }))
        .send()
        .await
        .unwrap();
    let file = WarpFile::from_owned_bytes(resp.bytes().await.unwrap().to_vec()).unwrap();
    let ChunkKind::Type(tc) = &file.chunks[0].kind else {
        panic!("expected a type chunk")
    };
    assert_eq!(
        tc.type_with_guid(&TypeGUID::from(&my_struct())),
        Some(my_struct())
    );

    let resp = p
        .app
        .get(&format!("/api/v1/types/{}", Uuid::new_v4()))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn search_across_kinds() {
    let p = pushed().await;

    let resp = p
        .app
        .get("/api/v1/search?q=alp&retrieve_data=true")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let result = body(resp).await;
    let items = result["items"].as_array().unwrap();
    // function "alpha", its type "alpha", and symbol "alpha".
    assert_eq!(result["total"], 3);
    let function = items
        .iter()
        .find(|i| i["kind"] == "function")
        .expect("function hit");
    assert_eq!(function["name"], "alpha");
    assert_eq!(function["source_id"], p.source.to_string());
    assert_eq!(
        function["function_guid"],
        mock_function_guid("alpha").to_string()
    );
    let parsed = Function::from_bytes(&bytes_of(&function["data"])).expect("function bytes");
    assert_eq!(parsed.symbol.name, "alpha");
    let ty = items
        .iter()
        .find(|i| i["kind"] == "type")
        .expect("type hit");
    assert!(Type::from_bytes(&bytes_of(&ty["data"])).is_some());

    // Without retrieve_data, payloads are omitted.
    let resp = p
        .app
        .get("/api/v1/search?q=alp&kind=function")
        .send()
        .await
        .unwrap();
    let result = body(resp).await;
    assert_eq!(result["total"], 1);
    assert!(result["items"][0]["data"].is_null());

    let resp = p
        .app
        .get("/api/v1/search?kind=source&q=lib")
        .send()
        .await
        .unwrap();
    let result = body(resp).await;
    assert_eq!(result["total"], 1);
    assert_eq!(result["items"][0]["name"], "libfoo");
    assert_eq!(result["items"][0]["id"], p.source.to_string());

    let resp = p
        .app
        .get(&format!(
            "/api/v1/search?function_guid={}",
            mock_function_guid("beta")
        ))
        .send()
        .await
        .unwrap();
    let result = body(resp).await;
    assert_eq!(result["total"], 1);
    assert_eq!(result["items"][0]["kind"], "function");

    let resp = p
        .app
        .get("/api/v1/search?source_tags[0]=official")
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total"], 0);

    let resp = p
        .app
        .get("/api/v1/search?limit=2&offset=1")
        .send()
        .await
        .unwrap();
    let result = body(resp).await;
    assert_eq!(result["limit"], 2);
    assert_eq!(result["offset"], 1);
    assert_eq!(result["items"].as_array().unwrap().len(), 2);

    let resp = p.app.get("/api/v1/search?kind=bogus").send().await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn deleting_a_commit_removes_its_data() {
    let p = pushed().await;
    let (_, other_key) = p
        .app
        .new_user("other@test.local", "other", Role::User)
        .await;

    let resp = p
        .app
        .post("/api/v1/commits/query")
        .json(&json!({ "source_id": p.source }))
        .send()
        .await
        .unwrap();
    let page = body(resp).await;
    assert_eq!(page["total_results"], 1);
    assert_eq!(page["items"][0]["id"], p.commit_id);
    assert_eq!(page["items"][0]["name"], "commit");

    let resp = p
        .app
        .get(&format!("/api/v1/commits/{}", p.commit_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = p
        .app
        .delete(&format!("/api/v1/commits/{}", p.commit_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let resp = p
        .app
        .delete(&format!("/api/v1/commits/{}", p.commit_id))
        .bearer_auth(&other_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
    let resp = p
        .app
        .delete(&format!("/api/v1/commits/{}", p.commit_id))
        .bearer_auth(&p.dev_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = p
        .app
        .get(&format!("/api/v1/commits/{}", p.commit_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    assert_eq!(query_functions(&p.app, json!({})).await["total_results"], 0);
    let resp = p
        .app
        .post("/api/v1/types/query")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total_results"], 0);
    let resp = p
        .app
        .get("/api/v1/search?kind=function")
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total"], 0);
}

#[tokio::test]
async fn deleting_a_source_removes_its_commits() {
    let p = pushed().await;

    let resp = p
        .app
        .delete(&format!("/api/v1/sources/{}", p.source))
        .bearer_auth(&p.dev_key)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    assert_eq!(query_functions(&p.app, json!({})).await["total_results"], 0);
    let resp = p
        .app
        .post("/api/v1/commits/query")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total_results"], 0);
}

#[tokio::test]
async fn push_rejections() {
    let app = spawn_app().await;
    let (_, dev_key) = app.new_user("dev@test.local", "dev", Role::User).await;
    let (_, other_key) = app.new_user("other@test.local", "other", Role::User).await;
    let source = app.create_source(&dev_key, "libfoo").await;
    let file = sample_file();

    let resp = app
        .post("/api/v1/files/json")
        .json(&json!({ "file": encode(&file), "name": "x", "source": source }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let resp = push(&app, &other_key, source, &file).await;
    assert_eq!(resp.status(), 403);

    let resp = push(&app, &dev_key, Uuid::new_v4(), &file).await;
    assert_eq!(resp.status(), 404);

    let resp = app
        .post("/api/v1/files/json")
        .bearer_auth(&dev_key)
        .json(&json!({ "file": "@@not base64@@", "name": "x", "source": source }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let garbage = base64::engine::general_purpose::STANDARD.encode(b"definitely not a warp file");
    let resp = app
        .post("/api/v1/files/json")
        .bearer_auth(&dev_key)
        .json(&json!({ "file": garbage, "name": "x", "source": source }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Admins may push anywhere.
    let resp = push(&app, &app.admin_key, source, &file).await;
    assert_eq!(resp.status(), 200);

    // Nothing from the failed attempts leaked in.
    let resp = app
        .post("/api/v1/commits/query")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(body(resp).await["total_results"], 1);
}

#[tokio::test]
async fn multipart_push() {
    let app = spawn_app().await;
    let (_, dev_key) = app.new_user("dev@test.local", "dev", Role::User).await;
    let source = app.create_source(&dev_key, "libfoo").await;

    let form = reqwest::multipart::Form::new()
        .text("name", "from multipart")
        .text("source", source.to_string())
        .text("description", "uploaded with curl -F")
        .part(
            "file",
            reqwest::multipart::Part::bytes(sample_file().to_bytes()).file_name("libfoo.warp"),
        );
    // The upstream spec spells this route `/api/v1/files/`.
    let resp = app
        .post("/api/v1/files/")
        .bearer_auth(&dev_key)
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "{}", resp.text().await.unwrap());
    let commit_id = body(resp).await["commit_id"].as_i64().unwrap();

    let resp = app
        .get(&format!("/api/v1/commits/{commit_id}"))
        .send()
        .await
        .unwrap();
    let commit = body(resp).await;
    assert_eq!(commit["name"], "from multipart");
    assert_eq!(commit["description"], "uploaded with curl -F");
    assert_eq!(
        query_functions(&app, json!({ "commit_id": commit_id })).await["total_results"],
        3
    );
}
