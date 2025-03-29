use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    value: Arc<str>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Pattern {
    Static(String),
    Dynamic(Param),
    Wildcard(Param),
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub(crate) fn patterns(path: &str) -> impl Iterator<Item = Pattern> + '_ {
    split(path).into_iter().map(|[start, end]| {
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
                Some(name) => Pattern::Dynamic(name.to_owned().into()),
            },

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => match segment.get(1..) {
                None | Some("") => panic!("Wildcard parameters must be named. Found '*'."),
                Some(name) => Pattern::Wildcard(name.to_owned().into()),
            },

            // The segment does not start with a reserved character. We will
            // consider it a static pattern that can be matched by value.
            _ => Pattern::Static(segment.into()),
        }
    })
}

pub(crate) fn split(path: &str) -> Vec<[usize; 2]> {
    let mut parts = Vec::with_capacity(6);
    let mut start = 0;

    let bytes = path.as_bytes();
    let len = bytes.len();

    for (index, byte) in bytes.iter().enumerate() {
        if path.is_char_boundary(index) && *byte == b'/' {
            if bytes.get(start).is_some_and(|from| *from != b'/') {
                parts.push([start, index]);
            }

            start = index + 1;
        }
    }

    if len > start {
        parts.push([start, len]);
    }

    parts
}

impl Display for Param {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl From<String> for Param {
    fn from(value: String) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl PartialEq<str> for Param {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        *self.value == *other
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

    fn get_expected_results() -> [Vec<[usize; 2]>; 16] {
        [
            vec![[1, 5], [6, 11]],
            vec![[1, 9], [10, 14], [15, 18]],
            vec![[1, 5], [6, 11], [12, 16], [17, 21]],
            vec![[1, 5], [6, 13], [14, 22]],
            vec![[1, 9], [10, 17]],
            vec![[1, 7], [8, 23]],
            vec![[1, 5], [6, 12]],
            vec![[1, 10], [11, 18]],
            vec![[1, 4]],
            vec![[1, 8], [9, 16]],
            vec![[2, 6], [8, 13]],
            vec![[1, 9], [11, 15], [16, 19]],
            vec![[1, 5], [6, 11], [12, 16], [18, 22]],
            vec![[1, 5], [7, 14], [15, 23]],
            vec![[1, 9], [10, 17], [19, 21]],
            vec![],
        ]
    }

    #[test]
    fn test_split_into() {
        let expected_results = get_expected_results();

        for (i, path) in PATHS.iter().enumerate() {
            let segments = super::split(path);

            assert_eq!(
                segments.len(),
                expected_results[i].len(),
                "split produced less segments than expected"
            );

            for (j, segment) in segments.into_iter().enumerate() {
                let expect = expected_results[i][j];
                assert_eq!(segment, expect, "{} ({}, {:?})", path, j, segment);
            }
        }
    }
}
