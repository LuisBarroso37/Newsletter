use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use sqlx::PgPool;

use crate::{domain::SubscriberEmail, email_client::EmailClient};

use super::subscriptions::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(thiserror::Error)]
pub enum PublishNewsletterError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishNewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishNewsletterError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            PublishNewsletterError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[tracing::instrument(
    name = "Publish newsletter to confirmed users",
    skip(body, connection_pool)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, PublishNewsletterError> {
    // Retrieve list of confirmed subscribers
    let confirmed_subscribers = get_confirmed_subscribers(&connection_pool)
        .await
        .context("Failed to retrieve confirmed subscribers.")?;

    // Send newsletter to each subscriber
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
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

    Ok(HttpResponse::Ok().finish())
}

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
