use http::StatusCode;
use percent_encoding::percent_decode_str;
use smallvec::SmallVec;
use std::{borrow::Cow, slice::Iter, str::FromStr};

use crate::{Error, Result};

type ValuesVec = SmallVec<[(usize, usize); 6]>;

pub struct QueryParamValue<'a, 'b> {
    name: &'b str,
    range: Option<(usize, usize)>,
    query: &'a str,
}

pub struct QueryParamValues<'a, 'b> {
    name: &'b str,
    query: &'a str,
    values: ValuesVec,
}

pub struct QueryParamValuesIter<'a> {
    inner: Iter<'a, (usize, usize)>,
    query: &'a str,
}

impl<'a, 'b> QueryParamValue<'a, 'b> {
    pub fn parse<T>(&self) -> Result<T>
    where
        Error: From<<T as FromStr>::Err>,
        T: FromStr,
    {
        self.require()?.parse().map_err(|error| {
            let mut error = Error::from(error);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }

    pub fn require(&self) -> Result<Cow<'a, str>> {
        if let Some((start, end)) = self.range {
            let raw_value = &self.query[start..end];
            let decoder = percent_decode_str(raw_value);

            return Ok(decoder.decode_utf8_lossy());
        }

        let mut error = Error::new(format!(
            "missing required query parameter: \"{}\"",
            self.name
        ));

        *error.status_mut() = StatusCode::BAD_REQUEST;
        Err(error)
    }
}

impl<'a, 'b> QueryParamValues<'a, 'b> {
    pub(super) fn new(name: &'b str, query: &'a str, values: ValuesVec) -> Self {
        QueryParamValues {
            name,
            query,
            values,
        }
    }

    pub fn get(&self, index: usize) -> QueryParamValue<'a, 'b> {
        self.value_at(self.values.get(index).copied())
    }

    pub fn first(&self) -> QueryParamValue<'a, 'b> {
        self.value_at(self.values.first().copied())
    }

    pub fn last(&self) -> QueryParamValue<'a, 'b> {
        self.value_at(self.values.last().copied())
    }

    pub fn iter(&self) -> QueryParamValuesIter {
        QueryParamValuesIter {
            inner: self.values.iter(),
            query: self.query,
        }
    }

    fn value_at(&self, range: Option<(usize, usize)>) -> QueryParamValue<'a, 'b> {
        QueryParamValue {
            range,
            name: self.name,
            query: self.query,
        }
    }
}

impl<'a> Iterator for QueryParamValuesIter<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, end) = *self.inner.next()?;
        let raw_value = &self.query[start..end];
        let decoder = percent_decode_str(raw_value);

        Some(decoder.decode_utf8_lossy())
    }
}
