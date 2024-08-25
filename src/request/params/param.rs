use http::StatusCode;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::str::FromStr;

use super::{DecodeParam, NoopDecoder};
use crate::{Error, Result};

pub struct Param<'a, 'b, T = NoopDecoder> {
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

    pub fn parse<F>(self) -> Result<F>
    where
        Error: From<<F as FromStr>::Err>,
        F: FromStr,
    {
        self.required()?.parse().map_err(|error| {
            let mut error = Error::from(error);
            let status = StatusCode::BAD_REQUEST;

            *error.status_mut() = status;
            error
        })
    }

    pub fn ok(self) -> Option<Cow<'a, str>> {
        self.required().ok()
    }

    pub fn required(self) -> Result<Cow<'a, str>, Error> {
        let name = self.name;
        let source = self.source;

        match self.at {
            Some(Some((start, end))) => T::decode(&source[start..end]),
            Some(None) | None => {
                let message = format!("missing required parameter: \"{}\"", name);
                let status = StatusCode::BAD_REQUEST;

                return Err(Error::with_status(message, status));
            }
        }
    }
}
