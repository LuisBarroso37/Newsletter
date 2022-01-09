use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Mock Postmark API
    // Even though we don't need the variable `mock_guard`, we still need to assign to it
    // otherwise the tests fail - Drop expectations are not performed
    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    // Create subscriber
    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    // Retrieve the confirmation link from the request received by the mock Postmark server
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;

    // Confirm user
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        // We assert that no request is fired at Postmark
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Act
    // A sketch of the newsletter payload structure
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let response = app.post_newsletters(newsletter_request_body).await;

    // Assert
    // Mock verifies on Drop that we haven't sent the newsletter email
    assert_eq!(response.status().as_u16(), 200);

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act
    // A sketch of the newsletter payload structure
    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>"
        }
    });

    let response = app.post_newsletters(newsletter_request_body).await;

    // Assert
    // Mock verifies on Drop that we have sent the newsletter email
    assert_eq!(response.status().as_u16(), 200);

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>"
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({ "title": "Newsletter!" }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_newsletters(invalid_body).await;

        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    // Arrange
    let app = spawn_app().await;

    // Random credentials
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    // Act
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    // Arrange
    let app = spawn_app().await;

    let username = &app.test_user.username;

    // Random password
    let password = Uuid::new_v4().to_string();
    assert_ne!(app.test_user.password, password);

    // Act
    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );

    // Teardown test database
    app.cleanup().await;
}
