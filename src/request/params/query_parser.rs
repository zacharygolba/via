use percent_encoding::percent_decode_str;
use std::borrow::Cow;
use std::iter::Peekable;
use std::str::CharIndices;

pub fn parse<'a>(
    chars: &mut Peekable<CharIndices>,
    query: &'a str,
) -> Option<(Cow<'a, str>, (usize, usize))> {
    parse_name(chars, query).zip(parse_value(chars, query))
}

fn decode(input: &str) -> Cow<str> {
    percent_decode_str(input).decode_utf8_lossy()
}

fn parse_name<'a>(chars: &mut Peekable<CharIndices>, query: &'a str) -> Option<Cow<'a, str>> {
    // Get the start index of the name by taking the next index from input.
    let start = take(chars)?;
    // Continue consuming the input until we reach the terminating equal sign.
    let end = take_until(chars, query, '=')?;

    // Move past and ignore any additional occurences of the equal sign.
    take_while(chars, query, '=');

    // Eagerly decode the percent-encoded characters in the name and return
    // an owned string.
    //
    // While returning an owned string can be more expensive, it simplifies the
    // API for the caller and allows us to cache the decoded result for fast lookups.
    Some(decode(&query[start..end]))
}

fn parse_value(chars: &mut Peekable<CharIndices>, query: &str) -> Option<(usize, usize)> {
    // Get the start index of the name by taking the next index from input.
    let start = take(chars)?;
    // Continue consuming the input until we reach the terminating ampersand.
    let end = take_until(chars, query, '&')?;

    // Move past and ignore any additional occurences of the ampersand character.
    take_while(chars, query, '&');

    // Return the start and end index of the query param value. The raw value
    // will be conditionally decoded by either the QueryParamValuesIter or
    // QueryParamValue type if the value is used.
    Some((start, end))
}

fn take(chars: &mut Peekable<CharIndices>) -> Option<usize> {
    chars.next().map(|(index, _)| index)
}

fn take_while(chars: &mut Peekable<CharIndices>, query: &str, predicate: char) -> Option<usize> {
    while let Some((index, next)) = chars.peek() {
        if predicate != *next {
            return Some(*index);
        }

        chars.next();
    }

    Some(query.len())
}

fn take_until(chars: &mut Peekable<CharIndices>, query: &str, predicate: char) -> Option<usize> {
    while let Some((index, next)) = chars.peek() {
        if predicate == *next {
            return Some(*index);
        }

        chars.next();
    }

    Some(query.len())
}

#[cfg(test)]
mod tests {
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
            let mut iter = query.char_indices().peekable();
            let mut entry_index = 0;
            let expected_result = &expected_results[expected_result_index];

            while let Some((name, at)) = super::parse(&mut iter, query) {
                let (expected_name, expected_at) = expected_result[entry_index];
                let expected_value = &query[expected_at.0..expected_at.1];
                let value = &query[at.0..at.1];

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

                entry_index += 1;
            }
        }
    }
}
