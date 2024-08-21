use http::StatusCode;
use std::str::FromStr;

use crate::{Error, Result};

//
// TODO:
//
// Create a new type for  `Vec<(&'static str, (usize, usize))>` to represent
// path parameters (and maybe query parameters). This would allow us to pass
// around references to other parts of the codebase without exposing the
// underlying implementation details.
//
pub type PathParams = Vec<(&'static str, (usize, usize))>;

pub struct PathParam<'a, 'b> {
    name: &'b str,
    path: &'a str,
    range: Option<&'a (usize, usize)>,
}

impl<'a, 'b> PathParam<'a, 'b> {
    pub(super) fn new(name: &'b str, path: &'a str, range: Option<&'a (usize, usize)>) -> Self {
        Self { name, path, range }
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

    pub fn optional(self) -> Option<&'a str> {
        self.range.map(|(start, end)| &self.path[*start..*end])
    }

    pub fn required(self) -> Result<&'a str> {
        let name = self.name;

        self.optional().ok_or_else(|| {
            Error::with_status(
                format!("missing required path parameter: \"{}\"", name),
                StatusCode::BAD_REQUEST,
            )
        })
    }
}
