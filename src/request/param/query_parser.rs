use percent_encoding::percent_decode_str;
use std::borrow::Cow;

type QueryParserOutput<'a> = (Cow<'a, str>, Option<(usize, Option<usize>)>);

pub struct QueryParser<'a> {
    input: &'a str,
    from: usize,
}

fn decode(input: &str) -> Cow<'_, str> {
    percent_decode_str(input).decode_utf8_lossy()
}

fn take_name(input: &str, from: usize) -> (usize, Option<Cow<'_, str>>) {
    let len = input.len();

    let at = take_while(input, from, |byte| byte == b'&').map(|start| {
        match take_while(input, start, |byte| byte != b'=') {
            Some(end) => (start, end),
            None => (start, len),
        }
    });

    match at {
        Some((start, end)) => (end, input.get(start..end).map(decode)),
        None => (len, None),
    }
}

fn take_value(input: &str, from: usize) -> (usize, Option<(usize, Option<usize>)>) {
    let len = input.len();

    let at = take_while(input, from, |byte| byte == b'=').map(|start| {
        match take_while(input, start, |byte| byte != b'&') {
            Some(end) => (start, Some(end)),
            None => (start, None),
        }
    });

    match at {
        Some((_, Some(end))) => (end, at),
        Some((_, None)) => (len, at),
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

    #[test]
    fn parse_query_params_test() {
        let query_strings = vec![
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
            // Invalid UTF-8 characters
            "query=books&category=%80%80%80",
            "query=books&category=%C3%28",
            "query%C3%28=books&category=%C3%28",
        ];

        let expected_results = vec![
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((21, Some(28)))),
                ("sort", Some((34, Some(37)))),
            ],
            vec![
                ("query", Some((6, Some(19)))),
                ("category", Some((29, Some(38)))),
            ],
            vec![
                ("category", Some((9, Some(14)))),
                ("category", Some((24, Some(35)))),
                ("category", Some((45, Some(53)))),
            ],
            vec![("query", Some((6, Some(11)))), ("category", None)],
            vec![
                ("query", Some((6, Some(22)))),
                ("category", Some((32, Some(36)))),
            ],
            vec![
                ("query", Some((6, Some(11)))),
                ("filter", Some((19, Some(45)))),
            ],
            vec![
                ("items[]", Some((8, Some(12)))),
                ("items[]", Some((21, Some(24)))),
                ("items[]", Some((33, Some(41)))),
            ],
            vec![],
            vec![("data", Some((5, Some(47))))],
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((21, Some(37)))),
            ],
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((22, Some(29)))),
            ],
            vec![
                ("query", Some((7, Some(12)))),
                ("category", Some((22, Some(29)))),
            ],
            vec![("query", Some((6, Some(11)))), ("category", None)],
            vec![("", Some((13, Some(20))))],
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((21, Some(28)))),
            ],
            vec![
                ("query", Some((8, Some(13)))),
                ("category", Some((25, Some(32)))),
            ],
            vec![
                ("query", Some((8, Some(13)))),
                ("category", Some((25, Some(32)))),
            ],
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((23, Some(32)))),
            ],
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((21, Some(30)))),
            ],
            vec![
                ("query", Some((6, Some(11)))),
                ("category", Some((21, Some(27)))),
            ],
            vec![
                ("query�(", Some((12, Some(17)))),
                ("category", Some((27, Some(33)))),
            ],
        ];

        for (expected_result_index, query) in query_strings.iter().enumerate() {
            let mut actual_result = vec![];
            let expected_result = &expected_results[expected_result_index];

            actual_result.extend(QueryParser::new(query));

            assert_eq!(
                actual_result.len(),
                expected_result.len(),
                "Expected {} to have {} entries. Got {} instead. Query: '{}'.",
                expected_result_index,
                expected_result.len(),
                actual_result.len(),
                query
            );

            for (entry_index, (name, at)) in actual_result.into_iter().enumerate() {
                let expect = &expected_result[entry_index];

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
