use http::StatusCode;
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    io::{Error as IoError, ErrorKind as IoErrorKind},
};

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
    #[cfg(feature = "json")]
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

    #[cfg(feature = "json")]
    pub fn json(mut self) -> Self {
        self.format = Some(Format::Json);
        self
    }
}

impl Error {
    pub(crate) fn into_response(self) -> Response {
        use http::header::HeaderValue;

        let result = match self.format {
            #[cfg(feature = "json")]
            Some(Format::Json) => {
                // The `serde` feature is enabled and the error format is `Json`.
                // Attempt to serialize the error as JSON. If serialization fails,
                // fallback to a plain text response.
                let status = self.status;
                Response::json(&self).status(status).finish()
            }
            _ => {
                // The `serde` feature is disabled or the error format is not `Json`.
                // Generate a plain text response by converting the error to a string.
                let message = self.to_string();
                let status = self.status;
                Response::text(message).status(status).finish()
            }
        };

        result.unwrap_or_else(|error| {
            if cfg!(debug_assertions) {
                //
                // TODO:
                //
                // Replace eprintln with pretty_env_logger or something
                // similar.
                //
                eprintln!("Error: {}", error);
            }

            // An error occurred while generating the response. Generate a
            // plain text response with the original error message and
            // return it without using the `ResponseBuilder`.
            let mut response = Response::new();
            let message = self.to_string();

            response.headers_mut().insert(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );

            if let Ok(length) = HeaderValue::from_str(&message.len().to_string()) {
                response
                    .headers_mut()
                    .insert(http::header::CONTENT_LENGTH, length);
            }

            *response.status_mut() = self.status;
            *response.body_mut() = message.into();

            response
        })
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

#[cfg(feature = "json")]
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
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

        impl serde::Serialize for SerializedError {
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
