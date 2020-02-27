use crate::{http::StatusCode, Respond, Response};
use ::failure::{Backtrace, Error as Reason, Fail};
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub struct Error {
    pub(crate) reason: Reason,
    pub(crate) response: Option<Response>,
}

impl Error {
    #[inline]
    pub fn response(reason: impl Into<Reason>, status: StatusCode, value: impl Respond) -> Error {
        let mut response = match value.respond() {
            Ok(value) => value,
            Err(e) => return e,
        };

        *response.status_mut() = status;
        Error {
            reason: reason.into(),
            response: Some(response),
        }
    }

    #[inline]
    pub fn backtrace(&self) -> &Backtrace {
        self.reason.backtrace()
    }
}

impl Display for Error {
    #[inline]
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.reason, formatter)
    }
}

impl<T: Fail> From<T> for Error {
    #[inline]
    fn from(fail: T) -> Error {
        Error {
            reason: fail.into(),
            response: None,
        }
    }
}
