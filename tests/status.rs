mod util;

use util::start_test_instance;



#[tokio::test]
async fn test_status(){ 
    let app = start_test_instance().await;

    let client = reqwest::Client::new();

    let response = client
            .get(&format!("{}/api/vi/users", &app.address))
            .send()
            .await
            .expect("Request failed");
        
    assert!(response.status().is_success());
    
}