use std::fmt;
use std::sync;

use actix_http::error::ResponseError;
use actix_session::Session;
use actix_web::{get, http, web, Responder};

use super::xid;
use crate::store;

/// The parameters passed in the path.
#[derive(Deserialize)]
pub struct Path {
    /// The name of the message.
    message_name: String,
}

/// The possible error values.
#[derive(Debug)]
pub enum Error {
    /// The message name is invalid.
    UnknownMessage,

    /// The room is unknown.
    UnknownRoom,
}

#[get("/{message_name}")]
pub async fn handle(
    path: web::Path<Path>,
    cache: web::Data<super::Cache>,
    _store: web::Data<sync::Arc<sync::Mutex<store::Store>>>,
    session: Session,
) -> impl Responder {
    if let Some(message) =
        cache.read().iter().find(|m| m.name == path.message_name)
    {
        message
            .lookup(super::assert_id(&session, || message.entry())?)
            .and_then(|pos| message.describe(pos))
            .map(web::Json)
            .ok_or(Error::UnknownRoom)
    } else {
        log::info!("Message {} does not exist", path.message_name);
        Err(Error::UnknownMessage)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownMessage => write!(f, "unknown message"),
            Error::UnknownRoom => write!(f, "unknown room"),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::UnknownMessage => http::StatusCode::NOT_FOUND,
            Error::UnknownRoom => http::StatusCode::NOT_FOUND,
        }
    }
}

impl From<xid::Error> for Error {
    fn from(_: xid::Error) -> Self {
        Self::UnknownRoom
    }
}
