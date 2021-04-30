use std::fmt;
use std::sync;

use actix_http::error::ResponseError;
use actix_session::Session;
use actix_web::{http, post, web, HttpRequest, HttpResponse, Responder};

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

/// The response.
#[derive(Debug)]
pub struct Response(String);

/// The possible error values.
#[derive(Debug)]
pub enum Error {
    /// The message is invalid.
    MessageInvalid,

    /// A message with the same name already exists.
    AlreadyExists,

    /// An internal error occurred.
    InternalError,
}

/// Creates a message.
///
/// # Arguments
/// *  `req` - A description of the message to create.
#[post("/")]
pub async fn handle(
    req: web::Json<Request>,
    store: web::Data<sync::Arc<sync::Mutex<store::Store>>>,
    session: Session,
) -> impl Responder {
    let mut store = store.lock()?;

    if req.text.len() > MAX_LENGTH || req.text.len() < 1 {
        log::info!("Invalid message: {}", req.text);
        Err(Error::MessageInvalid)
    } else {
        if store.exists(&req.name)? {
            Err(Error::AlreadyExists)
        } else {
            let req = req.into_inner();
            store.put_message(&super::Message::new(
                &req.name, &req.text, req.shape, req.seed,
            ))?;
            super::clear_id(&session);
            Ok(Response(req.name))
        }
    }
}

impl Responder for Response {
    type Error = actix_http::error::Error;
    type Future = HttpResponse;

    fn respond_to(self, request: &HttpRequest) -> Self::Future {
        let url = format!(
            "http://{}{}{}",
            request
                .headers()
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("host"),
            request.uri().path(),
            self.0,
        );
        log::info!("Created message with location {}", url);
        HttpResponse::Created()
            .header(http::header::LOCATION, url)
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MessageInvalid => write!(f, "message invalid"),
            Error::AlreadyExists => write!(f, "already exists"),
            Error::InternalError => write!(f, "internal error"),
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::MessageInvalid => http::StatusCode::BAD_REQUEST,
            Error::AlreadyExists => http::StatusCode::CONFLICT,
            Error::InternalError => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
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
