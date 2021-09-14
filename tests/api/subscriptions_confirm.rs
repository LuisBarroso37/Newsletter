use crate::helpers::spawn_app;

use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[actix_rt::test]
async fn confirmations_without_token_are_rejected_with_a_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[actix_rt::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Make request to /subscriptions
    app.post_subscriptions(body.into()).await;

    // Get the request sent to the email client server
    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    // Extract confirmation links from email request
    let confirmation_links = app.get_confirmation_links(email_request);

    // Send request to /subscriptions/confirm?subscriptiontoken=...
    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[actix_rt::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    // Make request to /subscriptions
    app.post_subscriptions(body.into()).await;

    // Get the request sent to the email client server
    let email_request = &app.email_server.received_requests().await.unwrap()[0];

    // Extract confirmation links from email request
    let confirmation_links = app.get_confirmation_links(email_request);

    // Send request to /subscriptions/confirm?subscriptiontoken=...
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved_subscriber = sqlx::query!("Select email, name, status FROM subscriptions")
        .fetch_one(&app.connection_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved_subscriber.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved_subscriber.name, "le guin");
    assert_eq!(saved_subscriber.status, "confirmed");
}
