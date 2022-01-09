use actix_session::Session;
use actix_session::SessionExt;
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::{ready, Ready};
use uuid::Uuid;

pub struct TypedSession(Session);

impl TypedSession {
    const USERS_ID_KEY: &'static str = "user_id";

    /// Renew session key
    pub fn renew(&self) {
        self.0.renew();
    }

    /// Insert user id in session state
    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), serde_json::Error> {
        self.0.insert(Self::USERS_ID_KEY, user_id)
    }

    /// Get user id from session state
    pub fn get_user_id(&self) -> Result<Option<Uuid>, serde_json::Error> {
        self.0.get(Self::USERS_ID_KEY)
    }

    /// Logout of admin dashboard
    pub fn log_out(self) {
        self.0.purge()
    }
}

impl FromRequest for TypedSession {
    // This is a complicated way of saying
    // "We return the same error returned by the
    // implementation of `FromRequest` for `Session`".
    type Error = <Session as FromRequest>::Error;

    // From request expects a `Future` as return type to allow for extractors
    // that need to perform asynchronous operations (e.g. a HTTP call)
    // We do not have a `Future`, because we don't perform any I/O,
    // so we wrap `TypedSession` into `Ready` to convert it into a `Future` that
    // resolves to the wrapped value the first time it's polled by the executor.
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
