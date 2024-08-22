use percent_encoding::percent_decode_str;
use std::borrow::Cow;
use std::slice::Iter;

use super::Param;

pub struct QueryParamValues<'a, 'b> {
    name: &'b str,
    query: &'a str,
    values: Option<Vec<&'a (usize, usize)>>,
}

pub struct QueryParamValuesIter<'a> {
    inner: Option<Iter<'a, &'a (usize, usize)>>,
    query: &'a str,
}

impl<'a, 'b> QueryParamValues<'a, 'b> {
    pub(crate) fn new(name: &'b str, query: &'a str, values: Vec<&'a (usize, usize)>) -> Self {
        Self {
            name,
            query,
            values: Some(values),
        }
    }

    pub(crate) fn empty(name: &'b str) -> Self {
        Self {
            name,
            query: "",
            values: None,
        }
    }

    pub fn get(&self, index: usize) -> Param<'a, 'b> {
        let at = self
            .values
            .as_ref()
            .and_then(|values| Some(*values.get(index)?));

        Param::new(at, self.name, self.query)
    }

    pub fn first(&self) -> Param<'a, 'b> {
        let at = self
            .values
            .as_ref()
            .and_then(|values| Some(*values.first()?));

        Param::new(at, self.name, self.query)
    }

    pub fn last(&self) -> Param<'a, 'b> {
        let at = self
            .values
            .as_ref()
            .and_then(|values| Some(*values.last()?));

        Param::new(at, self.name, self.query)
    }

    pub fn iter(&self) -> QueryParamValuesIter {
        QueryParamValuesIter {
            inner: self.values.as_ref().map(|values| values.iter()),
            query: self.query,
        }
    }
}

impl<'a> Iterator for QueryParamValuesIter<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, end) = **self.inner.as_mut()?.next()?;
        let raw_value = &self.query[start..end];
        let decoder = percent_decode_str(raw_value);

        Some(decoder.decode_utf8_lossy())
    }
}
