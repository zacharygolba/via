//! Conviently work with errors that may occur in an application.
//!

use http::StatusCode;
use serde::ser::SerializeStruct;
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
#[derive(Debug)]
pub struct Error {
    format: ErrorFormat,
    status: StatusCode,
    source: AnyError,
}

#[derive(Debug)]
enum ErrorFormat {
    Json,
    Text,
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

impl Error {
    /// Returns a new `Error` with the provided message.
    ///
    #[inline]
    pub fn new(source: AnyError) -> Self {
        Self {
            format: ErrorFormat::Text,
            status: StatusCode::INTERNAL_SERVER_ERROR,
            source,
        }
    }

    /// Returns a new [Error] that will be serialized as JSON when converted to
    /// a response.
    ///
    #[inline]
    pub fn as_json(self) -> Self {
        Self {
            format: ErrorFormat::Json,
            ..self
        }
    }

    /// Returns a new [Error] that will call the provided filter function when
    /// the error is converted to a response.
    ///
    /// If the provided filter function returns `None` value, the original error
    /// message will be included in the response body.
    ///
    pub fn with_message(self, message: impl ToString) -> Self {
        Self {
            source: Box::new(ErrorMessage {
                message: message.to_string(),
            }),
            ..self
        }
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
            format: ErrorFormat::Text,
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
        let mut response = match error.format {
            ErrorFormat::Text => Response::text(error.to_string()),
            ErrorFormat::Json => Response::json(&error).unwrap_or_else(|residual| {
                // Placeholder for tracing...
                if cfg!(debug_assertions) {
                    eprintln!("Error: {}", residual);
                }

                Response::text(error.to_string())
            }),
        };

        response.set_status(error.status);
        response
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Error", 1)?;
        let message = self.to_string();
        let errors = [ErrorMessage { message }];

        // Serialize the error as a single element array containing an object with
        // a message field. We do this to provide compatibility with popular API
        // specification formats like GraphQL and JSON:API.
        state.serialize_field("errors", &errors)?;
        state.end()
    }
}

impl StdError for ErrorMessage {}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}
