use crate::session_state::TypedSession;
use crate::utils::{internal_server_error, see_other_response};
use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    if session
        .get_user_id()
        .map_err(internal_server_error)?
        .is_none()
    {
        Ok(see_other_response("/login"))
    } else {
        session.log_out();
        FlashMessage::info("You have successfully logged out").send();
        Ok(see_other_response("/login"))
    }
}
