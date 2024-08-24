use percent_encoding::percent_decode_str;
use std::borrow::Cow;

type QueryParserOutput<'a> = (Cow<'a, str>, Option<(usize, usize)>);

pub struct QueryParser<'a> {
    input: &'a str,
    offset: usize,
}

impl<'a> QueryParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, offset: 0 }
    }

    pub fn input(&self) -> &'a str {
        self.input
    }
}

impl<'a> Iterator for QueryParser<'a> {
    type Item = QueryParserOutput<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (offset, param) = parse_param(self.input, self.offset);

        self.offset = offset;
        param
    }
}

fn decode(input: &str) -> Cow<str> {
    percent_decode_str(input).decode_utf8_lossy()
}

fn has_latin_char_at(input: &str, at: usize) -> bool {
    input.is_char_boundary(at) && input.is_char_boundary(at + 1)
}

fn parse_param(query: &str, from: usize) -> (usize, Option<QueryParserOutput>) {
    let (offset, name) = parse_name(query, from);
    let (offset, at) = parse_value(query, offset);
    let param = name.map(|name| {
        let (start, end) = at;
        let at = if start < end {
            Some((start, end))
        } else {
            None
        };

        (name, at)
    });

    (offset, param)
}

fn parse_name(input: &str, from: usize) -> (usize, Option<Cow<str>>) {
    // Get the start index of the name by taking the next index from input.
    let start = take_while(input, from, |byte| byte == b'&');
    // Continue consuming the input until we reach the terminating equal sign.
    let end = take_while(input, start, |byte| byte != b'=');
    // Get the decoded version of the name if the range is valid.
    let name = if start < end {
        Some(decode(&input[start..end]))
    } else {
        None
    };

    (end, name)
}

fn parse_value(input: &str, from: usize) -> (usize, (usize, usize)) {
    // Get the start index of the name by taking the next index from input.
    let start = take_while(input, from, |byte| byte == b'=');
    // Continue consuming the input until we reach the terminating ampersand.
    let end = take_while(input, start, |byte| byte != b'&');

    (end, (start, end))
}

fn take_while(input: &str, from: usize, f: impl Fn(u8) -> bool) -> usize {
    input
        .bytes()
        .enumerate()
        .skip(from)
        .find_map(|(index, byte)| {
            if has_latin_char_at(input, index) && f(byte) {
                None
            } else {
                Some(index)
            }
        })
        .unwrap_or(input.len())
}

#[cfg(test)]
mod tests {
    use super::QueryParser;

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

    fn get_expected_results() -> [Vec<(&'static str, (usize, usize))>; 21] {
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

        for (expected_result_index, query) in QUERY_STRINGS.iter().enumerate() {
            let expected_result = &expected_results[expected_result_index];

            for (entry_index, (name, at)) in QueryParser::new(query).enumerate() {
                let at = match at {
                    Some((start, end)) => (start, end),
                    None => continue,
                };

                let (expected_name, expected_at) = expected_result[entry_index];
                let expected_value = query
                    .get(expected_at.0..expected_at.1)
                    .unwrap_or_else(|| &query[expected_at.0..]);
                let value = query.get(at.0..at.1).unwrap_or_else(|| &query[at.0..]);

                assert_eq!(
                    name, expected_name,
                    "Expected name at index [{}, {}] to be {}. Got {} instead.",
                    expected_result_index, entry_index, expected_name, name
                );

                assert_eq!(
                    value, expected_value,
                    "Expected range at index [{}, {}] to be {}. Got {} instead.",
                    expected_result_index, entry_index, expected_value, value
                );
            }
        }
    }
}
