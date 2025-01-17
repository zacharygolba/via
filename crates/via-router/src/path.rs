#[derive(PartialEq)]
pub enum Pattern {
    Root,
    Static(String),
    Dynamic(String),
    Wildcard(String),
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    let mut segments = vec![];

    split(&mut segments, path);
    segments.into_iter().map(|(start, end)| {
        let segment = match path.get(start..end) {
            Some(slice) => slice,
            None => panic!("Path segments cannot be empty when defining a route."),
        };

        match segment.chars().next() {
            // Path segments that start with a colon are considered a Dynamic
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some(':') => match segment.get(1..) {
                None | Some("") => panic!("Dynamic parameters must be named. Found ':'."),
                Some(name) => Pattern::Dynamic(name.into()),
            },

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => match segment.get(1..) {
                None | Some("") => panic!("Wildcard parameters must be named. Found '*'."),
                Some(name) => Pattern::Wildcard(name.into()),
            },

            // The segment does not start with a reserved character. We will
            // consider it a static pattern that can be matched by value.
            _ => Pattern::Static(segment.into()),
        }
    })
}

/// Accumulate path segment ranges as a [Span] from path into segments.
///
pub fn split(segments: &mut Vec<(usize, usize)>, path: &str) {
    // Assume the byte position of the first span in path is `0`. Bounds checks
    // are required before creating a Span with this value.
    let mut start = 0;

    // The length of path could be the end offset of the last Span from path.
    let len = path.len();

    // Iterate over the byte position of each occurrence of '/' in path.
    for (end, _) in path.match_indices('/') {
        // If a range exists with a length greater than `0` from start to end,
        // append a Span to segments. This bounds check prevents an empty range
        // from ending up in segments.
        if end > start {
            segments.push((start, end));
        }

        // Move start to the byte position after end.
        start = end + 1;
    }

    // If a range exists with a length greater than `0` from start to the
    // length of path, append a Span to segments. This condition is true
    // when path does not end with `/`.
    if len > start {
        segments.push((start, len));
    }
}

#[cfg(test)]
mod tests {
    const PATHS: [&str; 16] = [
        "/home/about",
        "/products/item/123",
        "/blog/posts/2024/june",
        "/user/profile/settings",
        "/services/contact",
        "/search/results?q=books",
        "/news/latest",
        "/portfolio/designs",
        "/faq",
        "/support/tickets",
        "//home//about",
        "/products//item/123",
        "/blog/posts/2024//june",
        "/user//profile/settings",
        "/services/contact//us",
        "/",
    ];

    fn get_expected_results() -> [Vec<(usize, usize)>; 16] {
        [
            vec![(1, 5), (6, 11)],
            vec![(1, 9), (10, 14), (15, 18)],
            vec![(1, 5), (6, 11), (12, 16), (17, 21)],
            vec![(1, 5), (6, 13), (14, 22)],
            vec![(1, 9), (10, 17)],
            vec![(1, 7), (8, 23)],
            vec![(1, 5), (6, 12)],
            vec![(1, 10), (11, 18)],
            vec![(1, 4)],
            vec![(1, 8), (9, 16)],
            vec![(2, 6), (8, 13)],
            vec![(1, 9), (11, 15), (16, 19)],
            vec![(1, 5), (6, 11), (12, 16), (18, 22)],
            vec![(1, 5), (7, 14), (15, 23)],
            vec![(1, 9), (10, 17), (19, 21)],
            vec![],
        ]
    }

    #[test]
    fn test_split_into() {
        let expected_results = get_expected_results();

        for (path_index, path_value) in PATHS.iter().enumerate() {
            let mut segments = vec![];

            super::split(&mut segments, path_value);

            for (segment_index, segment_value) in segments.into_iter().enumerate() {
                assert_eq!(
                    segment_value, expected_results[path_index][segment_index],
                    "{} ({}, {:?})",
                    path_value, segment_index, segment_value
                );
            }
        }
    }
}
