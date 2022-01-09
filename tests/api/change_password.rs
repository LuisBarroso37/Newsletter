use crate::helpers::spawn_app;
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = app.get_change_password().await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("location").unwrap(), "/login");

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn new_password_is_invalid() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let test_cases = vec![
        (
            serde_json::json!({
                "current_password": &new_password,
                "new_password": "a".repeat(11),
                "new_password_check": &new_password,
            }),
            "new password too short",
            "New password should be at least 12 characters long",
        ),
        (
            serde_json::json!({
                "current_password": &new_password,
                "new_password": "a".repeat(128),
                "new_password_check": &new_password,
            }),
            "new password too long",
            "New password should be less than 128 characters long",
        ),
    ];

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    for (invalid_body, test_description, error_message) in test_cases {
        // Act - Part 2 - Try to change password
        let response = app.post_change_password(&invalid_body).await;

        // Assert
        assert_eq!(
            response.status().as_u16(),
            303,
            "The API was not successful with 303 See Other when {}.",
            test_description
        );
        assert_eq!(
            response.headers().get("location").unwrap(),
            "/admin/password",
            "The API was not successful with 303 See Other when {}.",
            test_description
        );

        // Act - Part 3 - Follow the redirect
        let response = app.get_change_password().await;

        // Assert
        let html_page = response.text().await.unwrap();

        assert!(
            html_page.contains(format!("<p><i>{}</i></p>", error_message).as_str()),
            "HTML did not contain correct error message when {}.",
            test_description
        );
    }

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Part 2 - Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &another_new_password,
        }))
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/admin/password"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_change_password().await;

    // Assert
    let html_page = response.text().await.unwrap();

    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
         the field values must match</i></p>"
    ));

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn current_password_must_match_stored_password() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    }))
    .await;

    // Act - Part 2 - Try to change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("location").unwrap(),
        "/admin/password"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_change_password().await;

    // Assert
    let html_page = response.text().await.unwrap();

    assert!(html_page.contains("<p><i>The current password is incorrect</i></p>"));

    // Teardown test database
    app.cleanup().await;
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    let response = app.post_login(&login_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );

    // Act - Part 2 - Change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/password"
    );

    // Act - Part 3 - Follow the redirect
    let response = app.get_change_password().await;
    let html_page = response.text().await.unwrap();

    // Assert
    assert!(html_page.contains("<p><i>Your password has been changed</i></p>"));

    // Act - Part 4 - Logout
    let response = app.post_logout().await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), "/login");

    // Act - Part 5 - Follow the redirect
    let response = app.get_login().await;
    let html_page = response.text().await.unwrap();

    // Assert
    assert!(html_page.contains("<p><i>You have successfully logged out</i></p>"));

    // Act - Part 6 - Login using the new password
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password
    });
    let response = app.post_login(&login_body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(
        response.headers().get("Location").unwrap(),
        "/admin/dashboard"
    );

    // Teardown test database
    app.cleanup().await;
}
