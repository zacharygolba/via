//! Error handling.
//!

mod raise;
mod rescue;
mod server;

use http::header;
use serde::{Serialize, Serializer};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Error as IoError};

#[doc(hidden)]
pub use http::StatusCode; // Required for the raise macro.

use crate::response::{Response, TEXT_PLAIN};

pub use rescue::{Rescue, Sanitizer};
pub(crate) use server::ServerError;

/// A type alias for `Box<dyn Error + Send + Sync>`.
///
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// An error type that can act as a specialized version of a
/// [`ResponseBuilder`](crate::response::ResponseBuilder).
///
#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    Message(String),
    Other(BoxError),
}

#[derive(Serialize)]
struct Errors<'a> {
    #[serde(serialize_with = "serialize_status_code")]
    status: StatusCode,
    errors: SmallVec<[ErrorMessage<'a>; 1]>,
}

#[derive(Serialize)]
struct ErrorMessage<'a> {
    message: Cow<'a, str>,
}

fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u16(status.as_u16())
}

impl Error {
    /// Returns a new error with the provided status and message.
    ///
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            kind: ErrorKind::Message(message.into()),
        }
    }

    /// Returns a new error with the provided status and source.
    ///
    pub fn from_source(status: StatusCode, source: BoxError) -> Self {
        Self {
            status,
            kind: ErrorKind::Other(source),
        }
    }

    /// Returns a new error with the provided source a status code derived from
    /// the [`ErrorKind`](io::ErrorKind).
    ///
    pub fn from_io_error(error: IoError) -> Self {
        let status = match error.kind() {
            // Implies a resource already exists.
            io::ErrorKind::AlreadyExists => StatusCode::CONFLICT,

            // Signals a broken connection.
            io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted => StatusCode::BAD_GATEWAY,

            // Suggests the service is not ready or available.
            io::ErrorKind::ConnectionRefused => StatusCode::SERVICE_UNAVAILABLE,

            // Generally indicates a malformed request.
            io::ErrorKind::InvalidData | io::ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,

            // Implies restricted access.
            io::ErrorKind::IsADirectory
            | io::ErrorKind::NotADirectory
            | io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,

            // Indicates a missing resource.
            io::ErrorKind::NotFound => StatusCode::NOT_FOUND,

            // Implies an upstream service timeout.
            io::ErrorKind::TimedOut => StatusCode::GATEWAY_TIMEOUT,

            // Any other kind is treated as an internal server error.
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self::from_source(status, Box::new(error))
    }

    /// Returns a reference to the error source.
    ///
    pub fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let ErrorKind::Other(source) = &self.kind {
            Some(&**source)
        } else {
            None
        }
    }

    fn repr_json(&self, status_code: StatusCode) -> Errors<'_> {
        let mut errors = Errors::new(status_code);

        if let ErrorKind::Message(message) = &self.kind {
            errors.push(Cow::Borrowed(message.as_str()));
        } else {
            let mut source = self.source();

            while let Some(error) = source {
                errors.push(Cow::Owned(error.to_string()));
                source = error.source();
            }

            // Reverse the order of the error messages to match the call stack.
            errors.reverse();
        }

        errors
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::Other(source) => Display::fmt(&**source, f),
            ErrorKind::Message(message) => Display::fmt(&**message, f),
        }
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(source: E) -> Self {
        Self::from_source(StatusCode::INTERNAL_SERVER_ERROR, Box::new(source))
    }
}

impl From<Error> for Response {
    fn from(error: Error) -> Self {
        let message = error.to_string();
        let content_len = message.len().into();

        let mut response = Self::new(message.into());
        *response.status_mut() = error.status;

        let headers = response.headers_mut();
        headers.insert(header::CONTENT_LENGTH, content_len);
        headers.insert(header::CONTENT_TYPE, TEXT_PLAIN);

        response
    }
}

impl<'a> Errors<'a> {
    pub(crate) fn new(status: StatusCode) -> Self {
        Self {
            status,
            errors: SmallVec::new(),
        }
    }

    pub(crate) fn push(&mut self, message: Cow<'a, str>) -> &mut Self {
        self.errors.push(ErrorMessage { message });
        self
    }

    fn reverse(&mut self) {
        self.errors.reverse();
    }
}
