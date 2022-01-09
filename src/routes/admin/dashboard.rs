use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;
use crate::utils::{internal_server_error, see_other_response};

pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(internal_server_error)? {
        get_username(user_id, &pool)
            .await
            .map_err(internal_server_error)?
    } else {
        return Ok(see_other_response("/login"));
    };

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
