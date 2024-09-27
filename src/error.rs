use http::StatusCode;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};
use std::io;

use crate::Response;

type AnyError = Box<dyn StdError + Send + Sync + 'static>;

pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type Source = (dyn StdError + 'static);

#[derive(Debug)]
pub struct Error {
    format: Option<Format>,
    source: AnyError,
    status: StatusCode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Json,
}

#[derive(Clone, Copy, Debug)]
struct Chain<'a> {
    source: Option<&'a (dyn StdError + 'static)>,
}

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

impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn StdError + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        self.source.map(|error| {
            self.source = error.source();
            error
        })
    }
}

impl Error {
    pub fn new(message: String) -> Self {
        Self {
            format: None,
            source: Box::new(ErrorMessage { message }),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn from_io_error(error: io::Error) -> Self {
        let status = match error.kind() {
            io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
            io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            format: None,
            source: Box::new(error),
            status,
        }
    }

    pub fn chain(&self) -> impl Iterator<Item = &Source> {
        Chain {
            source: Some(&*self.source),
        }
    }

    pub fn source(&self) -> &Source {
        &*self.source
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn status_mut(&mut self) -> &mut StatusCode {
        &mut self.status
    }

    pub fn json(mut self) -> Self {
        self.format = Some(Format::Json);
        self
    }
}

impl Error {
    pub(crate) fn into_response(mut self) -> Response {
        let mut response = loop {
            let result = match self.format {
                Some(Format::Json) => Response::json(&self),
                None => Ok(Response::text(self.to_string())),
            };

            match result {
                Ok(response) => break response,
                Err(error) => {
                    self.format = None;
                    if cfg!(debug_assertions) {
                        // TODO: Replace this with tracing.
                        eprintln!("Error: {}", error);
                    }
                }
            };
        };

        response.set_status(self.status);
        response
    }
}

impl Error {
    pub(crate) fn with_status(message: String, status: StatusCode) -> Self {
        Self {
            format: None,
            source: Box::new(ErrorMessage { message }),
            status,
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
    fn from(value: T) -> Self {
        Self {
            format: None,
            source: Box::new(value),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<Error> for AnyError {
    fn from(error: Error) -> Self {
        error.source
    }
}

impl From<Error> for Box<dyn StdError + Send> {
    fn from(error: Error) -> Self {
        error.source
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let message = self.to_string();
        let repr = SerializeError {
            errors: [ErrorMessage { message }],
        };

        repr.serialize(serializer)
    }
}

impl Debug for ErrorMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.message, f)
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

impl Serialize for SerializeError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SerializeError", 1)?;

        state.serialize_field("errors", &self.errors)?;
        state.end()
    }
}
