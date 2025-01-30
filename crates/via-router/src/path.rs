use std::fmt::{self, Display, Formatter};
use std::str::MatchIndices;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub enum Pattern {
    Root,
    Static(Param),
    Dynamic(Param),
    Wildcard(Param),
}

#[derive(Debug, Default, PartialEq)]
pub struct Param {
    value: Arc<str>,
}

pub struct Split<'a> {
    len: usize,
    offset: usize,
    indices: MatchIndices<'a, char>,
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    Split::new(path).map(|[start, end]| {
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
                Some(name) => Pattern::Dynamic(Param { value: name.into() }),
            },

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => match segment.get(1..) {
                None | Some("") => panic!("Wildcard parameters must be named. Found '*'."),
                Some(name) => Pattern::Wildcard(Param { value: name.into() }),
            },

            // The segment does not start with a reserved character. We will
            // consider it a static pattern that can be matched by value.
            _ => Pattern::Static(Param {
                value: segment.into(),
            }),
        }
    })
}

impl Clone for Param {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl<T> From<T> for Param
where
    Arc<str>: From<T>,
{
    fn from(value: T) -> Self {
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

impl<'a> Split<'a> {
    #[inline]
    pub fn new(path: &'a str) -> Self {
        Self {
            len: path.len(),
            offset: 0,
            indices: path.match_indices('/'),
        }
    }
}

impl Iterator for Split<'_> {
    type Item = [usize; 2];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let len = self.len;
        let offset = &mut self.offset;
        let indices = &mut self.indices;

        loop {
            let start = *offset;
            let end = match indices.next() {
                None if len > start => len,
                Some((index, _)) => index,
                None => return None,
            };

            // Move start to the byte position after end.
            *offset = end + 1;

            // If a range exists with a length greater than `0` from start to end,
            // append a Span to segments. This bounds check prevents an empty range
            // from ending up in segments.
            if end > start {
                return Some([start, end]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Split;

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
            for (j, segment) in Split::new(path).enumerate() {
                let expect = expected_results[i][j];
                assert_eq!(segment, expect, "{} ({}, {:?})", path, j, segment);
            }
        }
    }
}
