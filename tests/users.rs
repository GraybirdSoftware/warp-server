mod util;
use util::start_test_instance;
use warp_server::models::request::CreateUser;

#[tokio::test]
async fn test_create_user(){ 
    let app = start_test_instance().await;

    let client = reqwest::Client::new();


    let user = CreateUser {
        email:"tester@sometestingdomain.test".to_string(), 
        name: None 
    };

    let request = client
        .post(&format!("{}/api/v1/users", &app.address))
        .header("Content-Type", "application/json")
        .json(&user);

    let response = request.send().await.unwrap();
    dbg!(&response);
    assert!(response.status().is_success());            
}