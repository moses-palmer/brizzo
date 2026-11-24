use std::{fmt, sync};

use actix_session::Session;
use actix_web::{http, post, web, HttpResponse, Responder, ResponseError};

use maze;
use maze::initialize;

use crate::store;

/// The maximum length of a message.
const MAX_LENGTH: usize = 64;

/// A request to create a message.
#[derive(Deserialize, Serialize)]
pub struct Request {
    /// The name of the message.
    name: String,

    /// The actual message.
    text: String,

    /// The type of maze to generate.
    shape: maze::Shape,

    /// The random seed.
    seed: initialize::LFSR,
}

/// The possible error values.
#[derive(Debug)]
pub enum Error {
    /// The message is invalid.
    MessageInvalid,

    /// A message with the same name already exists.
    AlreadyExists,

    /// An internal error occurred.
    Internal,
}

/// Creates a message.
///
/// # Arguments
/// *  `req` - A description of the message to create.
#[post("/")]
pub async fn handle(
    req: web::Json<Request>,
    store: web::Data<sync::Mutex<store::Store>>,
    session: Session,
) -> impl Responder {
    let mut store = store.lock()?;

    if req.text.len() > MAX_LENGTH || req.text.is_empty() {
        log::info!("Invalid message: {}", req.text);
        Err(Error::MessageInvalid)
    } else {
        if store.exists(&req.name)? {
            Err(Error::AlreadyExists)
        } else {
            let req = req.into_inner();
            store.put_message(&super::Message::new(
                &req.name,
                &req.text,
                req.shape,
                req.seed.clone(),
            ))?;
            super::clear_id(&session);
            log::info!("Created message with location {}", req.name);
            Ok(HttpResponse::Created()
                .append_header((http::header::LOCATION, req.name))
                .finish())
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MessageInvalid => write!(f, "message invalid"),
            Error::AlreadyExists => write!(f, "already exists"),
            Error::Internal => write!(f, "internal error"),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::MessageInvalid => http::StatusCode::BAD_REQUEST,
            Error::AlreadyExists => http::StatusCode::CONFLICT,
            Error::Internal => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<store::Error> for Error {
    fn from(_source: store::Error) -> Self {
        Self::Internal
    }
}

impl<T> From<sync::PoisonError<T>> for Error {
    fn from(_source: sync::PoisonError<T>) -> Self {
        Self::Internal
    }
}
