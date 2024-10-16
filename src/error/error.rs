//! Conviently work with errors that may occur in an application.
//!

use http::StatusCode;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};

use super::{AnyError, Iter};
use crate::response::Response;

macro_rules! new_with_status {
    ($name:ident, $status:ident) => {
        #[inline]
        pub fn $name(source: crate::error::AnyError) -> crate::Error {
            let status = http::StatusCode::$status;
            crate::Error::new_with_status(source, status)
        }
    };
}

/// An error type that can be easily converted to a [`Response`].
///
#[derive(Debug, Serialize)]
pub struct Error {
    #[serde(skip)]
    json: bool,

    #[serde(skip)]
    filter: Option<fn(&str) -> Option<String>>,

    #[serde(skip)]
    status: StatusCode,

    #[serde(rename = "errors", serialize_with = "serialize_errors")]
    source: AnyError,
}

/// An error type that contains an error message stored in a string.
///
#[derive(Debug, Serialize)]
struct ErrorMessage {
    message: String,
}

new_with_status!(bad_request, BAD_REQUEST);
new_with_status!(not_found, NOT_FOUND);
new_with_status!(gateway_timeout, GATEWAY_TIMEOUT);
new_with_status!(internal_server_error, INTERNAL_SERVER_ERROR);

fn serialize_errors<S>(error: &AnyError, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut state = serializer.serialize_seq(Some(1))?;
    let message = ErrorMessage::new(error.to_string());

    state.serialize_element(&message)?;
    state.end()
}

impl Error {
    /// Returns a new `Error` with the provided message.
    ///
    #[inline]
    pub fn new(source: AnyError) -> Self {
        Self {
            json: false,
            filter: None,
            status: StatusCode::INTERNAL_SERVER_ERROR,
            source,
        }
    }

    /// Returns a new [Error] that will be serialized as JSON when converted to
    /// a response.
    ///
    #[inline]
    pub fn as_json(self) -> Self {
        Self { json: true, ..self }
    }

    /// Sets the status code of the response that will be generated from self.
    ///
    #[inline]
    pub fn with_status(self, status: StatusCode) -> Self {
        Self { status, ..self }
    }
}

impl Error {
    /// Returns an iterator over the sources of this error.
    ///
    pub fn iter(&self) -> Iter {
        Iter::new(Some(self.source()))
    }

    /// Returns the source of this error.
    ///
    pub fn source(&self) -> &(dyn StdError + 'static) {
        &*self.source
    }

    /// Returns a reference to the status code of this error.
    ///
    pub fn status(&self) -> &StatusCode {
        &self.status
    }
}

impl Error {
    #[inline]
    pub(crate) fn new_with_status(source: AnyError, status: StatusCode) -> Self {
        Self {
            json: false,
            filter: None,
            status,
            source,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.source, f)
    }
}

impl<T> From<T> for Error
where
    T: StdError + Send + Sync + 'static,
{
    #[inline]
    fn from(source: T) -> Self {
        Self::new(Box::new(source))
    }
}

impl From<Error> for Response {
    fn from(error: Error) -> Response {
        let mut response = if error.json {
            Response::json(&error).unwrap_or_else(|residual| {
                // Placeholder for tracing...
                if cfg!(debug_assertions) {
                    eprintln!("Error: {}", residual);
                }

                Response::text(error.to_string())
            })
        } else {
            Response::text(error.to_string())
        };

        response.set_status(error.status);
        response
    }
}

impl ErrorMessage {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl StdError for ErrorMessage {}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}
