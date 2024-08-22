use http::StatusCode;
use std::str::FromStr;

use crate::{Error, Result};

#[derive(Debug)]
pub enum ParamType {
    Path(&'static str, (usize, usize)),
    Query(String, (usize, usize)),
}

pub struct Param<'a, 'b> {
    at: Option<&'a (usize, usize)>,
    name: &'b str,
    source: &'a str,
}

impl<'a, 'b> Param<'a, 'b> {
    pub(crate) fn new(at: Option<&'a (usize, usize)>, name: &'b str, source: &'a str) -> Self {
        Self { at, name, source }
    }

    pub fn parse<T>(self) -> Result<T>
    where
        Error: From<<T as FromStr>::Err>,
        T: FromStr,
    {
        self.required()?.parse().map_err(|error| {
            let mut error = Error::from(error);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }

    pub fn ok(self) -> Option<&'a str> {
        let (start, end) = *self.at?;

        Some(&self.source[start..end])
    }

    pub fn required(self) -> Result<&'a str> {
        let name = self.name;

        self.ok().ok_or_else(|| {
            Error::with_status(
                format!("missing required parameter: \"{}\"", name),
                StatusCode::BAD_REQUEST,
            )
        })
    }
}

impl From<(&'static str, (usize, usize))> for ParamType {
    fn from((name, at): (&'static str, (usize, usize))) -> Self {
        Self::Path(name, at)
    }
}

impl From<(String, (usize, usize))> for ParamType {
    fn from((name, at): (String, (usize, usize))) -> Self {
        Self::Query(name, at)
    }
}
