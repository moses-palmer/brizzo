use std::error;
use std::fmt;
use std::io;

use r2d2_redis::r2d2;
use r2d2_redis::redis;

/// Errors relating to the store.
#[derive(Copy, Clone, Debug)]
pub enum Error {
    /// A connection error occurred.
    Connection,

    /// An internal error occurred.
    InternalError,

    /// A message exists.
    Exists,

    /// An error occurred while writing.
    ReadError,

    /// An error occurred while writing.
    WriteError,
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)?;
        Ok(())
    }
}

impl From<Error> for io::ErrorKind {
    fn from(source: Error) -> Self {
        match source {
            Error::Connection => Self::ConnectionRefused,
            _ => Self::Other,
        }
    }
}

impl From<Error> for io::Error {
    fn from(source: Error) -> Self {
        Self::new(source.into(), source)
    }
}

impl From<r2d2::Error> for Error {
    fn from(_source: r2d2::Error) -> Self {
        Self::Connection
    }
}

impl From<redis::RedisError> for Error {
    fn from(_source: redis::RedisError) -> Self {
        Self::Connection
    }
}
