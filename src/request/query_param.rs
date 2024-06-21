use http::StatusCode;
use percent_encoding::percent_decode_str;
use std::{borrow::Cow, slice::Iter, str::FromStr};

use crate::{Error, Result};

pub(crate) type QueryParams<T = String> = Vec<(T, (usize, usize))>;

type ValuesVec<'a> = Vec<&'a (usize, usize)>;

pub struct QueryParamValue<'a, 'b, 'c> {
    name: &'b str,
    range: Option<&'c (usize, usize)>,
    query: &'a str,
}

pub struct QueryParamValues<'a, 'b> {
    name: &'b str,
    query: &'a str,
    values: ValuesVec<'a>,
}

pub struct QueryParamValuesIter<'a> {
    inner: Iter<'a, &'a (usize, usize)>,
    query: &'a str,
}

impl<'a, 'b, 'c> QueryParamValue<'a, 'b, 'c> {
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

    pub fn required(self) -> Result<Cow<'a, str>> {
        if let Some((start, end)) = self.range {
            let raw_value = &self.query[*start..*end];
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
    pub(super) fn new(name: &'b str, query: &'a str, values: ValuesVec<'a>) -> Self {
        QueryParamValues {
            name,
            query,
            values,
        }
    }

    pub fn get(&self, index: usize) -> QueryParamValue<'a, 'b, '_> {
        self.value_at(self.values.get(index).map(|range| *range))
    }

    pub fn first(&self) -> QueryParamValue<'a, 'b, '_> {
        self.value_at(self.values.first().map(|range| *range))
    }

    pub fn last(&self) -> QueryParamValue<'a, 'b, '_> {
        self.value_at(self.values.last().map(|range| *range))
    }

    pub fn iter(&self) -> QueryParamValuesIter {
        QueryParamValuesIter {
            inner: self.values.iter(),
            query: self.query,
        }
    }

    fn value_at<'c>(&self, range: Option<&'c (usize, usize)>) -> QueryParamValue<'a, 'b, 'c> {
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
        let (start, end) = **self.inner.next()?;
        let raw_value = &self.query[start..end];
        let decoder = percent_decode_str(raw_value);

        Some(decoder.decode_utf8_lossy())
    }
}
