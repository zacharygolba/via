use crate::{http::StatusCode, respond, util, Respond, Response};
use serde_json::json;
use std::{
    convert::TryInto,
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
};

type DynError = Box<dyn StdError + Send>;
type Source = (dyn StdError + 'static);

pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Error {
    response: Option<Response>,
    source: Option<DynError>,
    value: DynError,
}

#[macro_export]
macro_rules! bail {
    ($($tokens:tt)*) => {{
        use $crate::error::Error;
        let value = format!($($tokens)*);
        return Err(Error::message(value));
    }};
}

#[derive(Debug)]
struct Bail {
    message: String,
}

struct Wrapped {
    error: Error,
}

impl Display for Bail {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.message, f)
    }
}

impl StdError for Bail {
    #[inline]
    fn source(&self) -> Option<&Source> {
        None
    }
}

impl Error {
    #[inline]
    pub fn message(value: impl Into<String>) -> Error {
        Error::from(Bail {
            message: value.into(),
        })
    }

    #[inline]
    pub fn chain(self, error: impl Into<DynError>) -> Error {
        Error {
            response: None,
            source: Some(self.into()),
            value: error.into(),
        }
    }

    #[inline]
    pub fn catch(self, responder: impl Respond) -> Error {
        match responder.respond().map(Some) {
            Ok(response) => Error { response, ..self },
            Err(error) => self.chain(error),
        }
    }

    #[inline]
    pub fn json(self) -> Error {
        if self.response.as_ref().map_or(false, util::is_json) {
            return self;
        }

        let body = respond::json(&json!({
            "error": {
                "message": self.to_string(),
            },
        }));

        self.catch(body)
    }

    #[inline]
    pub fn source(&self) -> Option<&Source> {
        if let Some(source) = &self.source {
            Some(&**source)
        } else {
            None
        }
    }

    #[inline]
    pub fn status(mut self, value: u16) -> Error {
        match &mut self.response {
            Some(response) => {
                *response.status_mut() = value.try_into().unwrap();
                self
            }
            None => {
                self.response = Some(Response::default());
                self.status(value)
            }
        }
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl<T> From<T> for Error
where
    T: StdError + Send + 'static,
{
    #[inline]
    fn from(value: T) -> Error {
        Error {
            response: None,
            source: None,
            value: Box::new(value),
        }
    }
}

impl From<Error> for Box<dyn StdError> {
    #[inline]
    fn from(error: Error) -> Box<dyn StdError> {
        Box::new(Wrapped { error })
    }
}

impl From<Error> for DynError {
    #[inline]
    fn from(error: Error) -> DynError {
        Box::new(Wrapped { error })
    }
}

impl From<Error> for Response {
    #[inline]
    fn from(error: Error) -> Response {
        let mut response = match error.response {
            Some(value) => return value,
            None => Response::new(format!("{}", error).into()),
        };

        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    }
}

impl Respond for Error {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        Ok(self.into())
    }
}

impl Debug for Wrapped {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.error, f)
    }
}

impl Display for Wrapped {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl StdError for Wrapped {
    #[inline]
    fn source(&self) -> Option<&Source> {
        self.error.source()
    }
}
