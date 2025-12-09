use std::borrow::Cow;

use super::path_param::PathParam;
use super::query_parser::QueryParser;
use crate::util::UriEncoding;

pub struct QueryParam<'a, 'b> {
    encoding: UriEncoding,
    parser: QueryParser<'a>,
    name: &'b str,
}

fn find_next(parser: &mut QueryParser, name: &str) -> Option<(usize, Option<usize>)> {
    parser.find_map(|(n, at)| if n == name { at } else { None })
}

impl<'a, 'b> QueryParam<'a, 'b> {
    /// Returns a new `QueryParam` that will percent-decode parameter values
    /// when an individual parameter is converted to a result.
    ///
    pub fn percent_decode(self) -> Self {
        Self {
            encoding: UriEncoding::Percent,
            ..self
        }
    }

    pub fn exists(mut self) -> bool {
        self.parser.any(|(n, _)| n == self.name)
    }

    pub fn first(mut self) -> PathParam<'a, 'b> {
        let name = self.name;
        let query = self.parser.input();

        PathParam::new(name, query, find_next(&mut self.parser, name))
    }

    pub fn last(self) -> PathParam<'a, 'b> {
        let query = self.parser.input();
        let name = self.name;

        PathParam::new(
            name,
            query,
            self.parser
                .filter_map(|(n, at)| if n == name { at } else { None })
                .last(),
        )
    }
}

impl<'a, 'b> QueryParam<'a, 'b> {
    pub(crate) fn new(name: &'b str, query: &'a str) -> Self {
        Self {
            encoding: UriEncoding::Unencoded,
            parser: QueryParser::new(query),
            name,
        }
    }
}

impl<'a> Iterator for QueryParam<'a, '_> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        let encoded = match find_next(&mut self.parser, self.name)? {
            (start, Some(end)) => self.parser.input().get(start..end)?,
            (start, None) => self.parser.input().get(start..)?,
        };

        self.encoding.decode(encoded).ok()
    }
}
