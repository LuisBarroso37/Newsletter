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

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    app.post_login(&login_body).await;

    // Act - Part 2 - Submit newsletter form

    let request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>"
    });

    let response = app.post_publish_newsletter(&request_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/newsletters"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_publish_newsletter().await;
    let html_page = response.text().await.unwrap();

    // Assert
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // Mock verifies on Drop that we haven't sent the newsletter email

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

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    app.post_login(&login_body).await;

    // Act - Part 2 - Submit newsletter form
    let request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>"
    });

    let response = app.post_publish_newsletter(&request_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/newsletters"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_publish_newsletter().await;
    let html_page = response.text().await.unwrap();

    // Assert
    assert!(html_page.contains("<p><i>The newsletter issue has been published!</i></p>"));
    // Mock verifies on Drop that we have sent the newsletter email

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
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>"
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "html": "<p>Newsletter body as HTML</p>"
            }),
            "missing text content",
        ),
        (
            serde_json::json!({
                "title": "Newsletter!",
                "text": "Newsletter body as plain text"
            }),
            "missing html content",
        ),
    ];

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    app.post_login(&login_body).await;

    // Act - Part 2 - Try to publish newsletter
    for (invalid_body, error_message) in test_cases {
        let response = app.post_publish_newsletter(&invalid_body).await;

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
async fn you_must_be_logged_in_to_see_the_newsletter_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_publish_newsletter().await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_a_newsletter() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let request_body = serde_json::json!({
        "title": "Newsletter title",
        "html": "<p>Newsletter body as HTML</p>",
        "text": "Newsletter body as plain text"
    });
    let response = app.post_publish_newsletter(&request_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");
}
