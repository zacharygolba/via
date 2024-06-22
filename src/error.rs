use http::StatusCode;
use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    io::{Error as IoError, ErrorKind as IoErrorKind},
};

use crate::{response::Response, IntoResponse};

type AnyError = Box<dyn StdError + Send + Sync + 'static>;

pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type Source = (dyn StdError + 'static);

#[derive(Debug)]
pub struct Error {
    format: Option<Format>,
    source: AnyError,
    status: StatusCode,
}

struct Bail {
    message: String,
}

#[derive(Clone, Copy, Debug)]
struct Chain<'a> {
    source: Option<&'a (dyn StdError + 'static)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    #[cfg(feature = "serde")]
    Json,
}

impl Bail {
    pub fn new(message: String) -> Bail {
        Bail { message }
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
        Error {
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

        Error {
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

    #[cfg(feature = "serde")]
    pub fn json(mut self) -> Self {
        self.format = Some(Format::Json);
        self
    }
}

impl Error {
    pub(crate) fn into_infallible_response(self) -> Response {
        use http::header::HeaderValue;

        let status_code = self.status;
        let message = self.to_string();

        match self.into_response() {
            Ok(response) => response,
            Err(error) => {
                let mut response = Response::new();

                response.headers_mut().insert(
                    http::header::CONTENT_TYPE,
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );

                if let Some(length) = HeaderValue::from_str(&message.len().to_string()).ok() {
                    response
                        .headers_mut()
                        .insert(http::header::CONTENT_LENGTH, length);
                }

                *response.status_mut() = status_code;
                *response.body_mut() = message.into();

                eprintln!("Failed to convert error into response: {}", error);

                response
            }
        }
    }
}

impl Error {
    pub(crate) fn with_status(message: String, status: StatusCode) -> Self {
        Error {
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

impl IntoResponse for Error {
    fn into_response(self) -> crate::Result<Response> {
        let response = match self.format {
            #[cfg(feature = "serde")]
            Some(Format::Json) => Response::json(&self),
            _ => Response::text(self.to_string()),
        };

        response.status(self.status).end()
    }
}

impl<T> From<T> for Error
where
    T: StdError + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Error {
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

#[cfg(feature = "serde")]
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
                SerializedError {
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
