use std::fmt;

use actix_http::error::ResponseError;
use actix_web::{http, post, web, HttpRequest, HttpResponse, Responder};

use maze;
use maze::initialize;

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
}

/// Creates a message.
///
/// # Arguments
/// *  `req` - A description of the message to create.
#[post("/")]
pub async fn handle(
    req: web::Json<Request>,
    cache: web::Data<super::Cache>,
) -> impl Responder {
    if req.text.len() > MAX_LENGTH || req.text.len() < 1 {
        log::info!("Invalid message: {}", req.text);
        Err(Error::MessageInvalid)
    } else {
        let req = req.into_inner();
        cache
            .store(super::Message::new(
                &req.name, &req.text, req.shape, req.seed,
            ))
            .map(Response)
            .map_err(|_| Error::AlreadyExists)
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
        }
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Error::MessageInvalid => http::StatusCode::BAD_REQUEST,
            Error::AlreadyExists => http::StatusCode::CONFLICT,
        }
    }
}
