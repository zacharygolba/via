use crate::{http::StatusCode, response::Response};
use serde::ser::{Serialize, Serializer};
use std::{
    collections::HashSet,
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
};

pub type AnyError = Box<dyn StdError + Send + Sync + 'static>;
pub type Result<T, E = Error> = std::result::Result<T, E>;
pub type Source = (dyn StdError + 'static);

pub trait ResultExt<T> {
    fn json(self) -> Result<T>;
    fn status(self, code: u16) -> Result<T>;
}

#[derive(Debug)]
pub struct Error {
    format: Option<Format>,
    source: AnyError,
    status: u16,
}

#[doc(hidden)]
pub struct Bail {
    pub(crate) message: String,
}

#[derive(Clone, Copy, Debug)]
struct Chain<'a> {
    source: Option<&'a (dyn StdError + 'static)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Json,
}

fn respond(error: Error) -> Result<Response> {
    let Error { format, status, .. } = error;
    let mut response = Response::new(match format {
        Some(Format::Json) => serde_json::to_vec(&error)?,
        None => error.to_string().into_bytes(),
    });

    *response.status_mut() = StatusCode::from_u16(status)?;
    Ok(response)
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
    pub fn chain(&self) -> impl Iterator<Item = &Source> {
        Chain {
            source: Some(&*self.source),
        }
    }

    pub fn json(mut self) -> Self {
        self.format = Some(Format::Json);
        self
    }

    pub fn source(&self) -> &Source {
        &*self.source
    }

    pub fn status(mut self, code: u16) -> Self {
        self.status = code;
        self
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
        Error {
            format: None,
            source: Box::new(value),
            status: 500,
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

        impl Serialize for SerializedError {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
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

impl From<Error> for Response {
    fn from(error: Error) -> Response {
        respond(error).unwrap_or_else(|e| {
            let mut response = Response::new("Internal Server Error");

            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            eprintln!("{}", e);
            response
        })
    }
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    Error: From<E>,
{
    fn json(self) -> Result<T, Error> {
        self.map_err(|e| Error::from(e).json())
    }

    fn status(self, code: u16) -> Result<T, Error> {
        self.map_err(|e| Error::from(e).status(code))
    }
}
