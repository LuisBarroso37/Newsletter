use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscriber",
    skip(parameters, connection_pool)
)]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    connection_pool: web::Data<PgPool>,
) -> Result<HttpResponse, HttpResponse> {
    let subscriber_id =
        get_subscriber_id_from_token(&parameters.subscription_token, &connection_pool)
            .await
            .map_err(|_| HttpResponse::InternalServerError().finish())?;

    // Check if subscriber id exists
    match subscriber_id {
        None => Err(HttpResponse::Unauthorized().finish()),
        Some(id) => {
            confirm_subscriber(id, &connection_pool)
                .await
                .map_err(|_| HttpResponse::InternalServerError().finish())?;

            Ok(HttpResponse::Ok().finish())
        }
    }
}

#[tracing::instrument(
    name = "Get subscriber_id from token",
    skip(subscription_token, connection_pool)
)]
pub async fn get_subscriber_id_from_token(
    subscription_token: &str,
    connection_pool: &PgPool,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token
    )
    .fetch_optional(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(
    name = "Mark a subscriber as confirmed",
    skip(subscriber_id, connection_pool)
)]
pub async fn confirm_subscriber(
    subscriber_id: Uuid,
    connection_pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
