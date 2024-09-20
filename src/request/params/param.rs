use http::StatusCode;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::str::FromStr;

use super::{DecodeParam, NoopDecode, PercentDecode};
use crate::{Error, Result};

pub struct Param<'a, 'b, T = NoopDecode> {
    at: Option<Option<(usize, usize)>>,
    name: &'b str,
    source: &'a str,
    _decode: PhantomData<T>,
}

fn missing_required_param<'a>(name: &str) -> Result<Cow<'a, str>, Error> {
    let message = format!("missing required parameter: \"{}\"", name);
    let status = StatusCode::BAD_REQUEST;

    Err(Error::with_status(message, status))
}

impl<'a, 'b, T: DecodeParam> Param<'a, 'b, T> {
    pub(crate) fn new(at: Option<Option<(usize, usize)>>, name: &'b str, source: &'a str) -> Self {
        Self {
            at,
            name,
            source,
            _decode: PhantomData,
        }
    }

    /// Returns a new `Param` that will percent-decode the parameter value
    /// before it is used.
    ///
    pub fn decode(self) -> Param<'a, 'b, PercentDecode> {
        Param {
            at: self.at,
            name: self.name,
            source: self.source,
            _decode: PhantomData,
        }
    }

    /// Calls [`str::parse`] on the parameter value if it exists and returns the
    /// result. If the param is encoded, it will be decoded before it is parsed.
    ///
    pub fn parse<U>(self) -> Result<U, Error>
    where
        Error: From<<U as FromStr>::Err>,
        U: FromStr,
    {
        self.into_result()?.parse().map_err(|error| {
            let mut error = Error::from(error);
            let status = StatusCode::BAD_REQUEST;

            *error.status_mut() = status;
            error
        })
    }

    /// Returns a result with the parameter value if it exists. If the param is
    /// encoded, it will be decoded before it is returned.
    ///
    /// # Errors
    ///
    /// If the parameter does not exist or could not be decoded with the
    /// implementation of `T::decode`, an error is returned with a 400 Bad
    /// Request status code.
    ///
    pub fn into_result(self) -> Result<Cow<'a, str>, Error> {
        self.at
            .and_then(|at| {
                let (start, end) = at?;
                self.source.get(start..end)
            })
            .map_or_else(
                || missing_required_param(self.name),
                |value| T::decode(value),
            )
    }
}
