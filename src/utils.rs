use actix_web::error::InternalError;
use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;

// Return an opaque 500 Internal Server Error while preserving the error's root cause for logging
pub fn internal_server_error<T>(e: T) -> actix_web::error::InternalError<T> {
    InternalError::from_response(e, HttpResponse::InternalServerError().finish())
}

// Return a 303 See Other response
pub fn see_other_response(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}
