use std::borrow::Cow;
use std::marker::PhantomData;

use super::decode::{DecodeParam, NoopDecode, PercentDecode};
use super::path_param::PathParam;
use super::query_parser::QueryParser;

pub struct QueryParam<'a, 'b, T = NoopDecode> {
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
    /// Returns a new `QueryParam` that will decode parameter values with
    /// `U::decode` when an individual parameter is converted to a result.
    ///
    pub fn decode<U: DecodeParam>(self) -> QueryParam<'a, 'b, U> {
        QueryParam {
            name: self.name,
            parser: self.parser,
            _decode: PhantomData,
        }
    }

    /// Returns a new `QueryParam` that will percent-decode parameter values
    /// when an individual parameter is converted to a result.
    ///
    pub fn percent_decode(self) -> QueryParam<'a, 'b, PercentDecode> {
        self.decode()
    }

    pub fn exists(mut self) -> bool {
        self.parser.any(|(n, _)| n == self.name)
    }

    pub fn first(mut self) -> PathParam<'a, 'b, T> {
        let name = self.name;
        let query = self.parser.input();

        PathParam::new(name, query, find_next(&mut self.parser, name))
    }

    pub fn last(self) -> PathParam<'a, 'b, T> {
        let query = self.parser.input();
        let name = self.name;

        PathParam::new(
            name,
            query,
            self.parser
                .filter_map(|(n, at)| if n == name { Some(at) } else { None })
                .last(),
        )
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

impl<'a, T: DecodeParam> Iterator for QueryParamIter<'a, '_, T> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, end) = find_next(&mut self.parser, self.name)??;
        let encoded = self.parser.input().get(start..end)?;

        T::decode(encoded).ok()
    }
}
