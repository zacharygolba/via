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

impl<'a, 'b, T: DecodeParam> Param<'a, 'b, T> {
    pub(crate) fn new(at: Option<Option<(usize, usize)>>, name: &'b str, source: &'a str) -> Self {
        Self {
            at,
            name,
            source,
            _decode: PhantomData,
        }
    }

    /// Returns a new `Param` that will decode the parameter value with
    /// `U::decode` when the parameter is converted to a result.
    ///
    pub fn decode<U: DecodeParam>(self) -> Param<'a, 'b, U> {
        Param {
            at: self.at,
            name: self.name,
            source: self.source,
            _decode: PhantomData,
        }
    }

    /// Returns a new `Param` that will percent-decode the parameter value with
    /// when the parameter is converted to a result.
    ///
    pub fn percent_decode(self) -> Param<'a, 'b, PercentDecode> {
        self.decode()
    }

    /// Calls [`str::parse`] on the parameter value if it exists and returns the
    /// result. If the param is encoded, it will be decoded before it is parsed.
    ///
    pub fn parse<U>(self) -> Result<U>
    where
        U: FromStr,
        U::Err: std::error::Error + Send + Sync + 'static,
    {
        match self.into_result()?.parse() {
            Ok(value) => Ok(value),
            Err(error) => {
                let source = Box::new(error);
                Err(Error::bad_request(source))
            }
        }
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
    pub fn into_result(self) -> Result<Cow<'a, str>> {
        match self.at.and_then(|at| {
            let (start, end) = at?;
            self.source.get(start..end)
        }) {
            Some(value) => T::decode(value),
            None => {
                let name = self.name;
                let message = format!("missing required parameter: \"{}\"", name);

                Err(Error::bad_request(message.into()))
            }
        }
    }
}
