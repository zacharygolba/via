use percent_encoding::percent_decode_str;
use std::borrow::Cow;

type QueryParserOutput<'a> = (Cow<'a, str>, Option<(usize, usize)>);

pub struct QueryParser<'a> {
    input: &'a str,
    from: usize,
}

fn decode(input: &str) -> Cow<str> {
    percent_decode_str(input).decode_utf8_lossy()
}

fn take_name(input: &str, from: usize) -> (usize, Option<Cow<str>>) {
    // Get the length of the input. We'll use this value as the end index if we
    // reach the end of the input.
    let len = input.len();
    // Get the start index of the name by finding the next byte that is not an
    // `&`. Then, map the index to a tuple containing both the start and end
    // index of the query parameter name.
    let at = take_while(input, from, |byte| byte == b'&').and_then(|start| {
        // Continue consuming the input until we reach the terminating `=`.
        match take_while(input, start, |byte| byte != b'=') {
            // If we find the terminating `=`, return the complete range.
            Some(end) => Some((start, end)),
            // Otherwise, return the start index and the length of the input.
            None => Some((start, len)),
        }
    });

    match at {
        Some((start, end)) => (end, input.get(start..end).map(decode)),
        None => (len, None),
    }
}

fn take_value(input: &str, from: usize) -> (usize, Option<(usize, usize)>) {
    // Get the length of the input. We'll use this value as the end index if we
    // reach the end of the input.
    let len = input.len();
    // Get the start index of the name by finding the next byte that is not an
    // `=`. Then, map the index to a tuple containing both the start and end
    // index of the query parameter value.
    let at = take_while(input, from, |byte| byte == b'=').map(|start| {
        // Continue consuming the input until we reach the terminating `&`.
        match take_while(input, start, |byte| byte != b'&') {
            // If we find the terminating `&`, return the complete range.
            Some(end) => (start, end),
            // Otherwise, return the start index and the length of the input.
            None => (start, len),
        }
    });

    match at {
        Some((_, end)) => (end, at),
        None => (len, None),
    }
}

fn take_while(input: &str, from: usize, f: impl Fn(u8) -> bool) -> Option<usize> {
    let rest = input.get(from..)?;

    rest.bytes()
        .enumerate()
        .find_map(|(to, byte)| if !f(byte) { from.checked_add(to) } else { None })
}

impl<'a> QueryParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, from: 0 }
    }

    pub fn input(&self) -> &'a str {
        self.input
    }
}

impl<'a> Iterator for QueryParser<'a> {
    type Item = QueryParserOutput<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, name) = take_name(self.input, self.from);
        let (end, at) = take_value(self.input, start);

        self.from = end;

        name.zip(Some(at))
    }
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

    fn get_expected_results() -> [Vec<(&'static str, Option<(usize, usize)>)>; 21] {
        [
            vec![
                ("query", Some((6, 11))),
                ("category", Some((21, 28))),
                ("sort", Some((34, 37))),
            ],
            vec![("query", Some((6, 19))), ("category", Some((29, 38)))],
            vec![
                ("category", Some((9, 14))),
                ("category", Some((24, 35))),
                ("category", Some((45, 53))),
            ],
            vec![("query", Some((6, 11))), ("category", None)],
            vec![("query", Some((6, 22))), ("category", Some((32, 36)))],
            vec![("query", Some((6, 11))), ("filter", Some((19, 45)))],
            vec![
                ("items[]", Some((8, 12))),
                ("items[]", Some((21, 24))),
                ("items[]", Some((33, 41))),
            ],
            vec![],
            vec![("data", Some((5, 47)))],
            vec![("query", Some((6, 11))), ("category", Some((21, 37)))],
            vec![("query", Some((6, 11))), ("category", Some((22, 29)))],
            vec![("query", Some((7, 12))), ("category", Some((22, 29)))],
            vec![("query", Some((6, 11))), ("category", None)],
            vec![("query", Some((6, 11))), ("", Some((13, 20)))],
            vec![("query", Some((6, 11))), ("category", Some((21, 28)))],
            vec![("query", Some((8, 13))), ("category", Some((25, 32)))],
            vec![("query", Some((8, 13))), ("category", Some((25, 32)))],
            vec![("query", Some((6, 11))), ("category", Some((23, 32)))],
            vec![("query", Some((6, 11))), ("category", Some((21, 30)))],
            vec![("query", Some((6, 11))), ("category", Some((21, 27)))],
            vec![("query�(", Some((12, 17))), ("category", Some((27, 33)))],
        ]
    }

    #[test]
    fn parse_query_params_test() {
        let expected_results = get_expected_results();

        for (expected_result_index, query) in QUERY_STRINGS.iter().enumerate() {
            let mut actual_result = vec![];
            let expected_result = &expected_results[expected_result_index];

            actual_result.extend(QueryParser::new(query));

            assert_eq!(
                actual_result.len(),
                expected_result.len(),
                "Expected {} to have {} entries. Got {} instead.\n\n{}\n\n",
                expected_result_index,
                expected_result.len(),
                actual_result.len(),
                query
            );

            for (entry_index, (name, at)) in actual_result.into_iter().enumerate() {
                let expect = expected_result[entry_index];

                assert_eq!(
                    name, expect.0,
                    "Expected name at index [{}, {}] to be {}. Got {} instead.",
                    expected_result_index, entry_index, expect.0, name
                );

                assert_eq!(
                    at, expect.1,
                    "Expected range at index [{}, {}] to be {:?}. Got {:?} instead.",
                    expected_result_index, entry_index, expect.1, at
                );
            }
        }
    }
}
