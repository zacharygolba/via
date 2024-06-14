use http::StatusCode;
use std::{collections::HashMap, str::FromStr};

use crate::{Error, Result};

pub type PathParams = HashMap<&'static str, (usize, usize)>;

#[derive(Clone, Copy, Debug)]
pub struct PathParam<'a> {
    name: &'a str,
    value: Option<&'a str>,
}

// TODO:
// Explore alternative ways to handle request parameters or take inspiration
// from the API of `std::option::Option` or `std::result::Result`.
impl<'a> PathParam<'a> {
    pub(super) fn new(name: &'a str, value: Option<&'a str>) -> Self {
        PathParam { name, value }
    }

    pub fn parse<T>(&self) -> Result<T>
    where
        Error: From<<T as FromStr>::Err>,
        T: FromStr,
    {
        Ok(self.require()?.parse()?)
    }

    pub fn expect(&self, message: &str) -> Result<&'a str> {
        self.value.ok_or_else(|| {
            let mut error = Error::new(message.to_owned());

            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }

    pub fn require(&self) -> Result<&'a str> {
        self.value.ok_or_else(|| {
            let mut error = Error::new(format!(
                "missing required path parameter: \"{}\"",
                self.name
            ));

            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }
}
