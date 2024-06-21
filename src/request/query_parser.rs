use percent_encoding::percent_decode_str;
use std::{iter::Peekable, str::CharIndices};

use super::query_param::QueryParams;

pub fn parse_query_params(input: &str) -> QueryParams {
    let mut query_params = Vec::with_capacity(16);
    let mut input = QueryParserInput::new(input);

    while let Some((name, value)) = parse_entry(&mut input) {
        query_params.push((name, value));
    }

    query_params
}

fn parse_name(input: &mut QueryParserInput) -> Option<String> {
    // Get the start index of the name by taking the next index from input.
    let start = input.take()?;
    // Continue consuming the input until we reach the terminating equal sign.
    let end = input.take_until('=')?;

    // Move past and ignore any additional occurences of the equal sign.
    input.take_while('=');

    // Eagerly decode the percent-encoded characters in the name and return
    // an owned string.
    //
    // While returning an owned string can be more expensive, it simplifies the
    // API for the caller and allows us to cache the decoded result for fast lookups.
    Some(
        percent_decode_str(&input.value[start..end])
            .decode_utf8_lossy()
            .into_owned(),
    )
}

fn parse_value(input: &mut QueryParserInput) -> Option<(usize, usize)> {
    // Get the start index of the name by taking the next index from input.
    let start = input.take()?;
    // Continue consuming the input until we reach the terminating ampersand.
    let end = input.take_until('&')?;

    // Move past and ignore any additional occurences of the ampersand character.
    input.take_while('&');

    // Return the start and end index of the query param value. The raw value
    // will be conditionally decoded by either the `QueryParamValuesIter` or
    // `QueryParamValue` type if the value is used.
    Some((start, end))
}

fn parse_entry(input: &mut QueryParserInput) -> Option<(String, (usize, usize))> {
    parse_name(input).zip(parse_value(input))
}

struct QueryParserInput<'a> {
    chars: Peekable<CharIndices<'a>>,
    value: &'a str,
}

impl<'a> QueryParserInput<'a> {
    fn new(value: &'a str) -> Self {
        Self {
            chars: value.char_indices().peekable(),
            value,
        }
    }

    fn take(&mut self) -> Option<usize> {
        self.chars.next().map(|(index, _)| index)
    }

    fn take_while(&mut self, predicate: char) -> Option<usize> {
        while let Some((index, next)) = self.chars.peek() {
            if predicate != *next {
                return Some(*index);
            }

            self.chars.next();
        }

        Some(self.value.len())
    }

    fn take_until(&mut self, predicate: char) -> Option<usize> {
        while let Some((index, next)) = self.chars.peek() {
            if predicate == *next {
                return Some(*index);
            }

            self.chars.next();
        }

        Some(self.value.len())
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_query_params, QueryParams};

    static QUERY_STRINGS: [&str; 21] = [
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

    fn get_expected_results() -> [QueryParams<&'static str>; 21] {
        [
            vec![
                ("query", (6, 11)),
                ("category", (21, 28)),
                ("sort", (34, 37)),
            ],
            vec![("query", (6, 19)), ("category", (29, 38))],
            vec![
                ("category", (9, 14)),
                ("category", (24, 35)),
                ("category", (45, 53)),
            ],
            vec![("query", (6, 11))],
            vec![("query", (6, 22)), ("category", (32, 36))],
            vec![("query", (6, 11)), ("filter", (19, 45))],
            vec![
                ("items[]", (8, 12)),
                ("items[]", (21, 24)),
                ("items[]", (33, 41)),
            ],
            vec![],
            vec![("data", (5, 47))],
            vec![("query", (6, 11)), ("category", (21, 37))],
            vec![("query", (6, 11)), ("category", (22, 29))],
            vec![("query", (7, 12)), ("category", (22, 29))],
            vec![("query", (6, 11))],
            vec![("query", (6, 11))],
            vec![("query", (6, 11)), ("category", (21, 28))],
            vec![("query", (8, 13)), ("category", (25, 32))],
            vec![("query", (8, 13)), ("category", (25, 32))],
            vec![("query", (6, 11)), ("category", (23, 32))],
            vec![("query", (6, 11)), ("category", (21, 30))],
            vec![("query", (6, 11)), ("category", (21, 27))],
            vec![("query�(", (12, 17)), ("category", (27, 33))],
        ]
    }

    #[test]
    fn parse_query_params_test() {
        let expected_results = get_expected_results();

        for (expected_result_index, query_string) in QUERY_STRINGS.iter().enumerate() {
            let expected_result = &expected_results[expected_result_index];
            let actual_result = parse_query_params(query_string);

            for (entry_index, entry_value) in actual_result.into_iter().enumerate() {
                assert_eq!(entry_value.0, expected_result[entry_index].0);
                assert_eq!(entry_value.1, expected_result[entry_index].1);
            }
        }
    }
}
