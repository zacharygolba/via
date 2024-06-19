use core::slice;
use http::StatusCode;
use percent_encoding::percent_decode_str;
use smallvec::SmallVec;
use std::{
    borrow::Cow,
    collections::HashMap,
    iter::Peekable,
    str::{CharIndices, FromStr},
};

use crate::{Error, Result};

pub(super) type ParsedQueryParams = HashMap<String, SmallVec<[(usize, usize); 4]>>;

pub struct QueryParamValues<'a, 'b> {
    name: &'b str,
    iter: Option<slice::Iter<'a, (usize, usize)>>,
    query: &'a str,
}

pub struct QueryParamValue<'a, 'b> {
    name: &'b str,
    range: Option<(usize, usize)>,
    query: &'a str,
}

pub struct QueryParams<'a> {
    params: &'a ParsedQueryParams,
    query: &'a str,
}

struct ParserInput<'a> {
    chars: Peekable<CharIndices<'a>>,
    value: &'a str,
}

pub fn parse(input: &str) -> ParsedQueryParams {
    let mut query = ParsedQueryParams::new();
    let mut input = ParserInput::new(input);

    while let Some((key, value)) = parse_key_value_pair(&mut input) {
        query.entry(key).or_default().push(value);
    }

    query
}

fn parse_key(input: &mut ParserInput) -> Option<String> {
    let (start, _) = *input.peek()?;
    let end = input
        .take_until(|char| char == '=')
        .unwrap_or_else(|| input.len());

    Some(
        percent_decode_str(&input.value[start..end])
            .decode_utf8_lossy()
            .into_owned(),
    )
}

fn parse_value(input: &mut ParserInput) -> Option<(usize, usize)> {
    let (start, _) = *input.peek()?;
    let end = input
        .take_until(|char| char == '&')
        .unwrap_or_else(|| input.len());

    input.take_until(|char| char != '&');
    Some((start, end))
}

fn parse_key_value_pair(input: &mut ParserInput) -> Option<(String, (usize, usize))> {
    parse_key(input).zip(parse_value(input))
}

impl<'a> QueryParams<'a> {
    pub(super) fn new(params: &'a ParsedQueryParams, query: &'a str) -> Self {
        QueryParams { params, query }
    }

    pub fn first<'b>(&self, key: &'b str) -> QueryParamValue<'a, 'b> {
        let range = self
            .params
            .get(key)
            .and_then(|values| values.first().copied());

        QueryParamValue {
            range,
            name: key,
            query: self.query,
        }
    }

    pub fn get<'b>(&self, key: &'b str) -> QueryParamValues<'a, 'b> {
        QueryParamValues {
            name: key,
            iter: self.params.get(key).map(|values| values.iter()),
            query: self.query,
        }
    }
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

    pub fn expect(&self, message: &str) -> Result<Cow<'a, str>> {
        let (start, end) = match self.range {
            Some(range) => range,
            None => {
                let mut error = Error::new(message.to_owned());

                *error.status_mut() = StatusCode::BAD_REQUEST;
                return Err(error);
            }
        };

        Ok(percent_decode_str(&self.query[start..end]).decode_utf8()?)
    }

    pub fn require(&self) -> Result<Cow<'a, str>> {
        let (start, end) = match self.range {
            Some(range) => range,
            None => {
                let mut error = Error::new(format!(
                    "missing required query parameter: \"{}\"",
                    self.name
                ));

                *error.status_mut() = StatusCode::BAD_REQUEST;
                return Err(error);
            }
        };

        Ok(percent_decode_str(&self.query[start..end]).decode_utf8()?)
    }
}

impl<'a> ParserInput<'a> {
    fn new(value: &'a str) -> Self {
        ParserInput {
            chars: value.char_indices().peekable(),
            value,
        }
    }

    fn len(&self) -> usize {
        self.value.len()
    }

    fn peek(&mut self) -> Option<&(usize, char)> {
        self.chars.peek()
    }

    fn take_while(&mut self, predicate: impl Fn(char) -> bool) -> Option<usize> {
        while let Some((index, char)) = self.peek() {
            if !predicate(*char) {
                return Some(*index);
            }

            self.next();
        }

        Some(self.len())
    }

    fn take_until(&mut self, predicate: impl Fn(char) -> bool) -> Option<usize> {
        while let Some((index, char)) = self.peek() {
            if predicate(*char) {
                return Some(*index);
            }

            self.next();
        }

        Some(self.len())
    }
}

impl<'a, 'b> Iterator for QueryParamValues<'a, 'b> {
    type Item = QueryParamValue<'a, 'b>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(QueryParamValue {
            name: self.name,
            range: self.iter.as_mut()?.next().copied(),
            query: self.query,
        })
    }
}

impl<'a> Iterator for ParserInput<'a> {
    type Item = (usize, char);

    fn next(&mut self) -> Option<Self::Item> {
        self.chars.next()
    }
}

#[cfg(test)]
mod tests {
    use crate::request::query_parser::QueryParams;

    static URL_QUERY_STRINGS: [&str; 21] = [
        "query=books&category=fiction&sort=asc",
        "query=hello%20world&category=%E2%9C%93",
        "category=books&category=electronics&category=clothing",
        "query=books&category=",
        "query=100%25%20organic&category=food",
        "query=books&filter={\"price\":\"low\",\"rating\":5}",
        "items[]=book&items[]=pen&items[]=notebook",
        "",
        "data={\"name\":\"John\",\"age\":30,\"city\":\"New York\"}",
        "query=books&category=fiction#section2",
        // Invalid query strings
        "query=books&&category=fiction", // Double ampersand
        "query==books&category=fiction", // Double equal sign
        "query=books&category",          // Key without value
        "query=books&=fiction",          // Value without key
        "query=books&category=fiction&", // Trailing ampersand
        // Percent-encoded keys
        "qu%65ry=books&ca%74egory=fiction",
        "%71uery=books&%63ategory=fiction",
        "query=books&ca%74egory=fic%74ion",
        // Invalid UTF-8 characters (using percent-encoded bytes that don't form valid UTF-8 sequences)
        "query=books&category=%80%80%80",    // Overlong encoding
        "query=books&category=%C3%28",       // Invalid UTF-8 sequence in value
        "query%C3%28=books&category=%C3%28", // Invalid UTF-8 sequence in key
    ];

    #[test]
    fn parse() {
        for query_string in URL_QUERY_STRINGS.iter() {
            let params = super::parse(query_string);
            let qp = QueryParams::new(&params, query_string);

            qp.first("query").require().unwrap();

            println!("{:?}", super::parse(query_string));
        }
    }
}
