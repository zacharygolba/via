use std::borrow::Cow;
use std::str::FromStr;

use crate::util::UriEncoding;
use crate::{Error, raise};

pub struct PathParam<'a, 'b> {
    encoding: UriEncoding,
    source: &'a str,
    name: &'b str,
    at: Option<(usize, Option<usize>)>,
}

impl<'a, 'b> PathParam<'a, 'b> {
    #[inline]
    pub(crate) fn new(name: &'b str, source: &'a str, at: Option<(usize, Option<usize>)>) -> Self {
        Self {
            encoding: UriEncoding::Unencoded,
            source,
            name,
            at,
        }
    }

    /// Returns a new `Param` that will percent-decode the parameter value with
    /// when the parameter is converted to a result.
    ///
    #[inline]
    pub fn percent_decode(self) -> Self {
        Self {
            encoding: UriEncoding::Percent,
            ..self
        }
    }

    /// Calls [`str::parse`] on the parameter value if it exists and returns the
    /// result. If the param is encoded, it will be decoded before it is parsed.
    ///
    #[inline]
    pub fn parse<U>(self) -> Result<U, Error>
    where
        U: FromStr,
        U::Err: std::error::Error + Send + Sync + 'static,
    {
        self.into_result()?
            .parse()
            .or_else(|error| raise!(400, error))
    }

    pub fn unwrap_or<U>(self, or: U) -> Cow<'a, str>
    where
        Cow<'a, str>: From<U>,
    {
        self.into_result().unwrap_or(or.into())
    }

    pub fn unwrap_or_else<U, F>(self, or_else: F) -> Cow<'a, str>
    where
        F: FnOnce(Error) -> U,
        Cow<'a, str>: From<U>,
    {
        self.into_result().unwrap_or_else(|e| or_else(e).into())
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
    #[inline]
    pub fn into_result(self) -> Result<Cow<'a, str>, Error> {
        match self.at {
            Some((start, Some(end))) => self.encoding.decode(&self.source[start..end]),
            Some((start, None)) => self.encoding.decode(&self.source[start..]),
            None => raise!(
                400,
                message = format!("Missing required parameter \"{}\".", self.name),
            ),
        }
    }
}
