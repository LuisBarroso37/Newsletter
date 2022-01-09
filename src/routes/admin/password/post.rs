use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::authentication::{update_password, validate_credentials, AuthError, Credentials};
use crate::domain::Password;
use crate::routes::admin::dashboard::get_username;
use crate::session_state::TypedSession;
use crate::utils::{internal_server_error, see_other_response};

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[tracing::instrument(name = "Change password", skip(form, pool, session))]
pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // Check if user is authenticated by trying to get user id from session
    let user_id = session.get_user_id().map_err(internal_server_error)?;

    if user_id.is_none() {
        return Ok(see_other_response("/login"));
    }

    let user_id = user_id.unwrap();

    // Validate new password
    let new_password = match Password::parse(form.new_password.expose_secret().clone()) {
        Ok(password) => password,
        Err(e) => {
            FlashMessage::error(e).send();

            return Ok(see_other_response("/admin/password"));
        }
    };

    // Check if new password verification fails
    if new_password.as_ref() != form.new_password_check.expose_secret() {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match",
        )
        .send();

        return Ok(see_other_response("/admin/password"));
    }

    // Get username linked to user id
    let username = get_username(user_id, &pool)
        .await
        .map_err(internal_server_error)?;

    let credentials = Credentials {
        username,
        password: form.0.current_password,
    };

    // Validate user credentials
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect").send();

                Ok(see_other_response("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(internal_server_error(e).into()),
        };
    }

    // Update user's password
    update_password(user_id, form.0.new_password, &pool)
        .await
        .map_err(internal_server_error)?;

    FlashMessage::info("Your password has been changed").send();

    Ok(see_other_response("/admin/password"))
}
