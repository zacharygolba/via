//! Conviently work with errors that may occur in an application.
//!

use http::StatusCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};
use std::io;

use crate::Response;

type Source = (dyn std::error::Error + 'static);

pub(crate) type AnyError = Box<dyn std::error::Error + Send + Sync>;

/// An error type that can be easily converted to a [`Response`].
///
#[derive(Debug)]
pub struct Error {
    format: Format,
    source: AnyError,
    status: StatusCode,
}

/// An iterator over the sources of an `Error`.
///
#[derive(Debug)]
pub struct Iter<'a> {
    source: Option<&'a Source>,
}

/// The format of the response body would be generated from an `Error`.
///
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Json,
    Text,
}

/// An error type that contains an error message stored in a string.
///
#[derive(Debug)]
struct ErrorMessage {
    message: String,
}

/// The serialized representation of an Error.
///
struct SerializeError {
    // Store the error message in an array. This makes it easier to work with
    // in client-side code.
    errors: [ErrorMessage; 1],
}

impl Error {
    /// Returns a new `Error` with the provided message.
    ///
    pub fn new(message: String) -> Self {
        Self {
            format: Format::Text,
            source: Box::new(ErrorMessage { message }),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn from_box_error(error: AnyError) -> Self {
        Self {
            format: Format::Text,
            source: error,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// A convience for creating a new `Error` from an [io::Error]. This method
    /// will be deprecated when specialization is released.
    ///
    pub fn from_io_error(error: io::Error) -> Self {
        let status = match error.kind() {
            io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
            io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            format: Format::Text,
            source: Box::new(error),
            status,
        }
    }

    /// Returns an iterator over the sources of this error.
    ///
    pub fn iter(&self) -> Iter {
        let source = self.source();

        Iter {
            source: Some(source),
        }
    }

    /// Returns the source of this error.
    ///
    pub fn source(&self) -> &Source {
        &*self.source
    }

    /// Returns a reference to the status code of this error.
    ///
    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    /// Returns a mutable reference to the status code of this error.
    ///
    pub fn status_mut(&mut self) -> &mut StatusCode {
        &mut self.status
    }

    /// Sets the status code of the error to the provided status code. Returns
    /// a reference to the updated status code.
    ///
    /// If the status code is not in thhe `400-599` range, the status code will
    /// not be updated.
    ///
    pub fn set_status(&mut self, status: StatusCode) -> &StatusCode {
        if status.is_client_error() || status.is_server_error() {
            self.status = status;
        }

        &self.status
    }

    /// Configures self to respond with JSON when converted to a response.
    ///
    pub fn respond_with_json(&mut self) {
        self.format = Format::Json;
    }
}

impl Error {
    pub(crate) fn new_with_status(message: String, status: StatusCode) -> Self {
        Self {
            format: Format::Text,
            source: Box::new(ErrorMessage { message }),
            status,
        }
    }

    pub(crate) fn into_response(self) -> Response {
        let mut format = self.format;
        let status = self.status;

        loop {
            let result = match format {
                Format::Json => Response::json(&self),
                Format::Text => Ok(Response::new(self.to_string().into())),
            };

            match result {
                Ok(mut response) => {
                    response.set_status(status);
                    return response;
                }
                Err(error) => {
                    // If the error could not be serialized to the requested format,
                    // use a plain text response instead.
                    format = Format::Text;
                    // Placeholder for tracing...
                    if cfg!(debug_assertions) {
                        // TODO: Replace this with tracing.
                        eprintln!("Error: {}", error);
                    }
                }
            }
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
    AnyError: From<T>,
{
    fn from(value: T) -> Self {
        Self {
            format: Format::Text,
            source: value.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// impl From<Error> for AnyError {
//     fn from(error: Error) -> Self {
//         error.source
//     }
// }

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let message = self.to_string();
        let repr = SerializeError {
            errors: [ErrorMessage { message }],
        };

        repr.serialize(serializer)
    }
}

impl Display for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}

impl StdError for ErrorMessage {}

impl Serialize for ErrorMessage {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("ErrorMessage", 1)?;

        state.serialize_field("message", &self.message)?;
        state.end()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a dyn StdError;

    fn next(&mut self) -> Option<Self::Item> {
        // Attempt to get a copy of the source error from self. If the source
        // field is None, return early.
        let next = self.source?;

        // Set self.source to the next source error.
        self.source = next.source();

        // Return the next error.
        Some(next)
    }
}

impl Serialize for SerializeError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SerializeError", 1)?;

        state.serialize_field("errors", &self.errors)?;
        state.end()
    }
}
