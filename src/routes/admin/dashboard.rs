use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::UserId;
use crate::utils::internal_server_error;

pub async fn admin_dashboard(
    user_id: web::ReqData<UserId>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = get_username(*user_id.into_inner(), &pool)
        .await
        .map_err(internal_server_error)?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
        <head>
            <meta charset="utf-8" />
            <meta name="viewport" content="width=device-width, initial-scale=1" />
            <title>Admin Dashboard</title>
        </head>
        <html lang="en">
        <body>
            <p>Welcome {}!</p>
            <p>Available actions:</p>
            <ol>
                <li><a href="/admin/password">Change password</a></li>
                <li><a href="/admin/newsletters">Publish a newsletter issue</a></li>
                <li>
                    <a href="javascript:document.logoutForm.submit()">Logout</a>
                    <form name="logoutForm" action="/admin/logout" method="post" hidden>
                        <input hidden type="submit" value="Logout">
                    </form>
                </li>
            </ol>
        </body>
        </html>"#,
            username
        )))
}

#[tracing::instrument(name = "Get username", skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let row = sqlx::query!(
        r#"
        SELECT username
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_one(pool)
    .await
    .context("Failed to perform query to retrieve a username")?;

    Ok(row.username)
}
