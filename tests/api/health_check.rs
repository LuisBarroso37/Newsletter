use crate::helpers::spawn_app;

#[tokio::test]
async fn health_check_works() {
    // Start server
    let app = spawn_app().await;

    // Start client
    let client = reqwest::Client::new();

    // Make request to server
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request");

    // Check that we get back a 200 response with no body
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    // Teardown test database
    app.cleanup().await;
}
