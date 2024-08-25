use std::borrow::Cow;

use super::{query_parser::QueryParser, DecodeParam, Param, PercentDecoder};

pub struct QueryParam<'a, 'b> {
    name: &'b str,
    parser: QueryParser<'a>,
}

pub struct QueryParamIter<'a, 'b> {
    name: &'b str,
    parser: QueryParser<'a>,
}

fn find_next(parser: &mut QueryParser, name: &str) -> Option<Option<(usize, usize)>> {
    parser.find_map(|(n, at)| if n == name { Some(at) } else { None })
}

impl<'a, 'b> QueryParam<'a, 'b> {
    pub fn exists(self) -> bool {
        let QueryParam { name, mut parser } = self;

        parser.any(|(n, _)| n == name)
    }

    pub fn first(self) -> Param<'a, 'b, PercentDecoder> {
        let QueryParam { name, mut parser } = self;
        let query = parser.input();
        let at = find_next(&mut parser, name);

        Param::new(at, name, query)
    }

    pub fn last(self) -> Param<'a, 'b, PercentDecoder> {
        let QueryParam { name, parser } = self;
        let query = parser.input();
        let at = parser
            .filter_map(|(n, at)| if n == name { Some(at) } else { None })
            .last();

        Param::new(at, name, query)
    }
}

impl<'a, 'b> QueryParam<'a, 'b> {
    pub(crate) fn new(name: &'b str, query: &'a str) -> Self {
        Self {
            name,
            parser: QueryParser::new(query),
        }
    }
}

impl<'a> Iterator for QueryParamIter<'a, '_> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let parser = &mut self.parser;
        let query = parser.input();
        let name = self.name;
        let at = find_next(parser, name)??;

        PercentDecoder::decode(&query[at.0..at.1]).ok()
    }
}

impl<'a, 'b> IntoIterator for QueryParam<'a, 'b> {
    type Item = Cow<'a, str>;
    type IntoIter = QueryParamIter<'a, 'b>;

    fn into_iter(self) -> Self::IntoIter {
        QueryParamIter {
            name: self.name,
            parser: self.parser,
        }
    }
}
