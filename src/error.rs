use http::StatusCode;
use serde::{Serialize, Serializer};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use crate::response::Response;

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

struct Bail {
    message: String,
}

#[derive(Clone, Copy, Debug)]
struct Chain<'a> {
    source: Option<&'a (dyn StdError + 'static)>,
}

impl Bail {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl Debug for Bail {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.message, f)
    }
}

impl Display for Bail {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}

impl StdError for Bail {}

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
            source: Box::new(Bail::new(message)),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn from_io_error(error: IoError) -> Self {
        let status = match error.kind() {
            IoErrorKind::NotFound => StatusCode::NOT_FOUND,
            IoErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
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
            source: Box::new(Bail::new(message)),
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

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        use std::collections::HashSet;

        #[derive(Eq, PartialEq, Hash)]
        struct SerializedError {
            message: String,
        }

        impl<'a> From<&'a Source> for SerializedError {
            fn from(error: &'a Source) -> Self {
                Self {
                    message: error.to_string(),
                }
            }
        }

        impl Serialize for SerializedError {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut state = serializer.serialize_struct("Error", 1)?;

                state.serialize_field("message", &self.message)?;
                state.end()
            }
        }

        let errors: HashSet<_> = self.chain().map(SerializedError::from).collect();
        let mut state = serializer.serialize_struct("Errors", 1)?;

        state.serialize_field("errors", &errors)?;
        state.end()
    }
}

impl From<Error> for Box<dyn StdError + Send> {
    fn from(error: Error) -> Self {
        error.source
    }
}
