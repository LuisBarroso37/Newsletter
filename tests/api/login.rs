use crate::helpers::spawn_app;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act - Part 1 - Try to login
    let body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = app.post_login(&body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("location").unwrap(), "/login");

    // Act - Part 2 - Follow the redirect to /login
    let response = app.get_login().await;

    // Assert
    let html_page = response.text().await.unwrap();
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Act - Part 3 - Reload the login page
    let response = app.get_login().await;

    // Assert
    let html_page = response.text().await.unwrap();
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    // Arrange
    let app = spawn_app().await;

    // Act - Part 1 - Login
    let body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });

    let response = app.post_login(&body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/admin/dashboard"
    );

    // Act - Part 2 - Follow the redirect
    let response = app.get_admin_dashboard().await;
    let html_page = response.text().await.unwrap();

    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Teardown test database
    app.cleanup().await;
}
