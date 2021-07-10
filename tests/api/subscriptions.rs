use crate::helpers::spawn_app;

use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_rt::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Start server
    let app = spawn_app().await;

    // Request body
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Mock Postmark API
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[actix_rt::test]
async fn subscribe_persists_the_new_subscriber() {
    // Start server
    let app = spawn_app().await;

    // Request body
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;

    let saved_subscriber = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.connection_pool)
        .await
        .expect("Failed to fetch saved subscriber");

    assert_eq!(saved_subscriber.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved_subscriber.name, "le guin");
    assert_eq!(saved_subscriber.status, "pending_confirmation");
}

#[actix_rt::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Setup
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}

#[actix_rt::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Setup
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both the name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[actix_rt::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    app.post_subscriptions(body.into()).await;

    // Assertions - Mock asserts on drop
}

#[actix_rt::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    // Start server
    let app = spawn_app().await;

    // Request body
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Send request
    app.post_subscriptions(body.into()).await;

    // Get the first intercepted request
    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    // Extract confirmation links from email request
    let confirmation_links = app.get_confirmation_links(&email_request);

    // The two links should be identical
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[actix_rt::test]
async fn subscribe_fails_if_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions DROP COLUMN email",)
        .execute(&app.connection_pool)
        .await
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500);
}
