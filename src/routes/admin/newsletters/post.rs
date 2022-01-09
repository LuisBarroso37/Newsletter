use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::session_state::TypedSession;
use crate::utils::{internal_server_error, see_other_response};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    html: String,
    text: String,
}

#[tracing::instrument(
    name = "Publish newsletter to confirmed users",
    skip(form, connection_pool, email_client, session),
    fields(user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    form: web::Json<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if user is authenticated by trying to get user id from session
    let user_id = session.get_user_id().map_err(internal_server_error)?;

    if user_id.is_none() {
        return Ok(see_other_response("/login"));
    }

    // Record user id in tracing span
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id.unwrap()));

    // Retrieve list of confirmed subscribers
    let confirmed_subscribers = get_confirmed_subscribers(&connection_pool)
        .await
        .map_err(internal_server_error)?;

    // Send newsletter to each subscriber
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &form.title, &form.html, &form.text)
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(internal_server_error)?;
            }
            Err(error) => {
                tracing::warn!(
                    // We record the error chain as a structured field
                    // on the log record
                    error.cause_chain = ?error,
                    "Skipping a confirmed subscriber. \
                    Their stored contact details are invalid."
                );
            }
        }
    }

    FlashMessage::info("The newsletter issue has been published!").send();
    Ok(see_other_response("/admin/newsletters"))
}

// Data obtained from query in 'get_confirmed_subscribers'
struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Fetch list of confirmed subscribers", skip(connection_pool))]
async fn get_confirmed_subscribers(
    connection_pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, sqlx::Error> {
    let rows = sqlx::query!(r#"SELECT email FROM subscriptions WHERE status = 'confirmed'"#)
        .fetch_all(connection_pool)
        .await?;

    // Map into the domain type
    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(e) => Err(anyhow::anyhow!(e)),
        })
        .collect();

    Ok(confirmed_subscribers)
}
