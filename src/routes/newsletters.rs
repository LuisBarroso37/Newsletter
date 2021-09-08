use actix_web::http::{header, HeaderMap, HeaderValue, StatusCode};
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sqlx::PgPool;

use crate::telemetry::spawn_blocking_with_tracing;
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
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishNewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishNewsletterError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishNewsletterError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                // Add 'WWW-Authenticta header to response
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);

                response
            }
            PublishNewsletterError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[tracing::instrument(
    name = "Publish newsletter to confirmed users",
    skip(body, connection_pool, email_client, request),
    fields(
        username=tracing::field::Empty,
        user_id=tracing::field::Empty
    )
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: web::HttpRequest,
) -> Result<HttpResponse, PublishNewsletterError> {
    // Extract user credentials from 'Authorization' header
    let credentials =
        basic_authentication(request.headers()).map_err(PublishNewsletterError::AuthError)?;

    // Record username in tracing span
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    // Validate user credentials
    let user_id = validate_credentials(credentials, &connection_pool).await?;

    // Record user id in tracing span
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

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

// User credentials used for authentication
struct Credentials {
    username: String,
    password: String,
}

/// Extract user credentials from 'Authorization' header
fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    // Extract 'Authorization' header
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;

    // Remove the prefix of the 'Authorization' header
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    // Decode base64 'Basic' credentials
    let decoded_bytes = base64::decode_config(base64encoded_segment, base64::STANDARD)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    // Convert vector of bytes into string
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid UTF8.")?;

    // Split decoded credentials into two segments, using ':' as delimiter
    let mut credentials = decoded_credentials.splitn(2, ':');

    // Extract username and password
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided in 'Basic' auth."))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be provided in 'Basic' auth."))?
        .to_string();

    Ok(Credentials { username, password })
}

#[tracing::instrument(name = "Validate user credentials", skip(credentials, connection_pool))]
async fn validate_credentials(
    credentials: Credentials,
    connection_pool: &PgPool,
) -> Result<uuid::Uuid, PublishNewsletterError> {
    // Set fallback user id and password hash in case no user is found with given username
    // We do this to return an error later in order to avoid timing attacks
    let mut user_id = None;
    let mut expected_password_hash = "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
        .to_string();

    // Update user's id and password hash if user exists with given username
    if let Some((stored_user_id, stored_password_hash)) =
        get_stored_credentials(&credentials.username, connection_pool)
            .await
            .map_err(PublishNewsletterError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_password_hash;
    }

    // Parse password hash
    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishNewsletterError::UnexpectedError)?
        .to_string();

    // Verify if passwords match
    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")
    .map_err(PublishNewsletterError::UnexpectedError)??;

    // This is only set to `Some` if we found credentials in the database.
    // So, even if the default password ends up matching (somehow)
    // with the provided password,
    // We never authenticate a non-existing user.
    user_id.ok_or_else(|| PublishNewsletterError::AuthError(anyhow::anyhow!("Unknown username.")))
}

#[tracing::instrument(name = "Get stored credentials", skip(username, connection_pool))]
async fn get_stored_credentials(
    username: &str,
    connection_pool: &PgPool,
) -> Result<Option<(uuid::Uuid, String)>, anyhow::Error> {
    let row = sqlx::query!(
        r#"SELECT id, password FROM users WHERE username = $1"#,
        username
    )
    .fetch_optional(connection_pool)
    .await
    .context("Failed to perform a query to retrieve stored credentials.")?
    .map(|row| (row.id, row.password));

    Ok(row)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: String,
    password_candidate: String,
) -> Result<(), PublishNewsletterError> {
    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("Failed to parse hash in PHC string format.")
        .map_err(PublishNewsletterError::UnexpectedError)?;

    Argon2::default()
        .verify_password(password_candidate.as_bytes(), &expected_password_hash)
        .context("Invalid password.")
        .map_err(PublishNewsletterError::AuthError)
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
