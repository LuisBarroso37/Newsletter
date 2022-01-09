use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

use crate::session_state::TypedSession;
use crate::utils::{internal_server_error, see_other_response};

pub async fn change_password_form(
    session: TypedSession,
    flash_messages: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if user is authenticated
    if session
        .get_user_id()
        .map_err(internal_server_error)?
        .is_none()
    {
        return Ok(see_other_response("/login"));
    }

    // Retrieve all flash messages
    let mut msg_html = String::new();

    for msg in flash_messages.iter() {
        writeln!(msg_html, "<p><i>{}</i></p>", msg.content()).unwrap();
    }

    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Change Password</title>
</head>
<html lang="en">
<body>
    {}
    <form action="/admin/password" method="post">
        <label>Current password
            <input
                type="password"
                placeholder="Enter current password"
                name="current_password"
            />
        </label>
        <br />
        <label>New password
            <input
                type="password"
                placeholder="Enter new password"
                name="new_password"
            />
        </label>
        <br />
        <label>Confirm new password
            <input
                type="password"
                placeholder="Type the new password again"
                name="new_password_check"
            />
        </label>
        <br />
        <button type="submit">Change password</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
            msg_html
        )))
}
