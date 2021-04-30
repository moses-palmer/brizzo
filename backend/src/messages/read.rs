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
    /// The message is unknown.
    UnknownMessage,

    /// The room is unknown.
    UnknownRoom,

    /// An internal error occurred.
    InternalError,
}

#[get("/{message_name}")]
pub async fn handle(
    path: web::Path<Path>,
    store: web::Data<sync::Arc<sync::Mutex<store::Store>>>,
    session: Session,
) -> impl Responder {
    let mut store = store.lock()?;

    if !store.exists(&path.message_name)? {
        Err(Error::UnknownMessage)
    } else {
        let current_id = match super::load_id(&session) {
            Ok(id) => Some(id),
            Err(xid::Error::Expired) | Err(xid::Error::Missing) => None,
            Err(e) => return Err(e.into()),
        };
        store
            .get(&path.message_name, current_id)?
            .ok_or(Error::UnknownRoom)
            .map(web::Json)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnknownMessage => write!(f, "unknown message"),
            Error::UnknownRoom => write!(f, "unknown room"),
            Error::InternalError => write!(f, "internal error"),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::UnknownMessage => http::StatusCode::NOT_FOUND,
            Error::UnknownRoom => http::StatusCode::NOT_FOUND,
            Error::InternalError => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<xid::Error> for Error {
    fn from(_: xid::Error) -> Self {
        Self::UnknownRoom
    }
}

impl From<store::Error> for Error {
    fn from(_source: store::Error) -> Self {
        Self::InternalError
    }
}

impl<T> From<sync::PoisonError<T>> for Error {
    fn from(_source: sync::PoisonError<T>) -> Self {
        Self::InternalError
    }
}
