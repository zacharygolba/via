use std::borrow::Cow;
use std::marker::PhantomData;

use super::query_parser::QueryParser;
use super::{DecodeParam, NoopDecoder, Param};

pub struct QueryParam<'a, 'b, T = NoopDecoder> {
    name: &'b str,
    parser: QueryParser<'a>,
    _decode: PhantomData<T>,
}

pub struct QueryParamIter<'a, 'b, T> {
    name: &'b str,
    parser: QueryParser<'a>,
    _decode: PhantomData<T>,
}

fn find_next(parser: &mut QueryParser, name: &str) -> Option<Option<(usize, usize)>> {
    parser.find_map(|(n, at)| if n == name { Some(at) } else { None })
}

impl<'a, 'b, T: DecodeParam> QueryParam<'a, 'b, T> {
    pub fn exists(mut self) -> bool {
        self.parser.any(|(n, _)| n == self.name)
    }

    pub fn first(mut self) -> Param<'a, 'b, T> {
        let query = self.parser.input();
        let name = self.name;
        let at = find_next(&mut self.parser, name);

        Param::new(at, name, query)
    }

    pub fn last(self) -> Param<'a, 'b> {
        let query = self.parser.input();
        let name = self.name;
        let at = self
            .parser
            .filter_map(|(n, at)| if n == name { Some(at) } else { None })
            .last();

        Param::new(at, name, query)
    }
}

impl<'a, 'b, T: DecodeParam> QueryParam<'a, 'b, T> {
    pub(crate) fn new(name: &'b str, query: &'a str) -> Self {
        Self {
            name,
            parser: QueryParser::new(query),
            _decode: PhantomData,
        }
    }
}

impl<'a, T: DecodeParam> Iterator for QueryParamIter<'a, '_, T> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, end) = find_next(&mut self.parser, self.name)??;
        let encoded = self.parser.input().get(start..end)?;

        T::decode(encoded).ok()
    }
}

impl<'a, 'b, T: DecodeParam> IntoIterator for QueryParam<'a, 'b, T> {
    type Item = Cow<'a, str>;
    type IntoIter = QueryParamIter<'a, 'b, T>;

    fn into_iter(self) -> Self::IntoIter {
        QueryParamIter {
            name: self.name,
            parser: self.parser,
            _decode: PhantomData,
        }
    }
}
