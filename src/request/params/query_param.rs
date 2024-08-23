use std::str::CharIndices;
use std::{borrow::Cow, iter::Peekable};

use super::{query_parser, DecodeParam, Param, PercentDecoder};

pub struct QueryParam<'a, 'b> {
    name: &'b str,
    query: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

pub struct QueryParamIter<'a, 'b> {
    name: &'b str,
    query: &'a str,
    chars: Peekable<CharIndices<'a>>,
}

fn find_next(chars: &mut Peekable<CharIndices>, query: &str, name: &str) -> Option<(usize, usize)> {
    loop {
        let (next, at) = query_parser::parse(chars, query)?;

        if name == next {
            return Some(at);
        }
    }
}

impl<'a, 'b> QueryParam<'a, 'b> {
    pub fn first(mut self) -> Param<'a, 'b, PercentDecoder> {
        let chars = &mut self.chars;
        let query = self.query;
        let name = self.name;

        Param::new(find_next(chars, query, name), name, query)
    }

    pub fn last(mut self) -> Param<'a, 'b, PercentDecoder> {
        let mut at = None;
        let chars = &mut self.chars;
        let query = self.query;
        let name = self.name;

        while let Some(next) = find_next(chars, query, name) {
            at = Some(next);
        }

        Param::new(at, self.name, query)
    }
}

impl<'a, 'b> QueryParam<'a, 'b> {
    pub(crate) fn new(name: &'b str, query: &'a str) -> Self {
        let chars = query.char_indices().peekable();

        Self { name, query, chars }
    }
}

impl<'a> Iterator for QueryParamIter<'a, '_> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let query = self.query;
        let name = self.name;
        let (start, end) = find_next(&mut self.chars, query, name)?;

        PercentDecoder::decode(&query[start..end]).ok()
    }
}

impl<'a, 'b> IntoIterator for QueryParam<'a, 'b> {
    type Item = Cow<'a, str>;
    type IntoIter = QueryParamIter<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        QueryParamIter {
            name: self.name,
            query: self.query,
            chars: self.chars,
        }
    }
}
