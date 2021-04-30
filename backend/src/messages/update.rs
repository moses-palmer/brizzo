use std::fmt;

use actix_http::error::ResponseError;
use actix_session::Session;
use actix_web::{http, put, web, Responder};

use super::xid;

/// The parameters passed in the path.
#[derive(Deserialize)]
pub struct Path {
    /// The name of the message.
    message_name: String,
}

/// A request to create a message.
#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    /// The identifier of the room to which to move.
    xid: xid::Identifier,
}

#[derive(Debug)]
pub enum Error {
    /// The message name is invalid.
    UnknownMessage,

    /// The room is unknown.
    UnknownRoom,

    /// The specified transition is illegal.
    IllegalTransition,
}

#[put("/{message_name}")]
pub async fn handle(
    path: web::Path<Path>,
    req: web::Json<Request>,
    cache: web::Data<super::Cache>,
    session: Session,
) -> impl Responder {
    if let Some(message) =
        cache.read().iter().find(|m| m.name == path.message_name)
    {
        let current = super::assert_id(&session, || message.entry())?;
        if let Some(next) = message
            .lookup(current)
            .and_then(|pos| message.transition(pos, req.xid))
        {
            super::store_id(&session, req.xid)?;
            message
                .describe(next)
                .ok_or(Error::UnknownRoom)
                .map(web::Json)
        } else {
            log::info!("Cannot transition from {} to {}", current, req.xid);
            Err(Error::IllegalTransition)
        }
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
            Error::IllegalTransition => write!(f, "illegal transition"),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::UnknownMessage => http::StatusCode::NOT_FOUND,
            Error::UnknownRoom => http::StatusCode::NOT_FOUND,
            Error::IllegalTransition => http::StatusCode::NOT_FOUND,
        }
    }
}

impl From<xid::Error> for Error {
    fn from(_: xid::Error) -> Self {
        Self::UnknownRoom
    }
}
