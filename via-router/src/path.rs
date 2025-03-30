use std::fmt::{self, Display, Formatter};
use std::iter::Peekable;
use std::mem;
use std::str::MatchIndices;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    value: Arc<str>,
}

pub struct Split<'a> {
    len: usize,
    offset: usize,
    indices: MatchIndices<'a, char>,
}

pub struct SplitWithLookahead<'a> {
    split: Peekable<Split<'a>>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Pattern {
    Root,
    Static(String),
    Dynamic(Param),
    Wildcard(Param),
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub(crate) fn patterns(path: &str) -> impl Iterator<Item = Pattern> + '_ {
    Split::new(path).into_iter().map(|[start, end]| {
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

impl<'a> Split<'a> {
    pub fn new(path: &'a str) -> Self {
        Self {
            len: path.len(),
            offset: 0,
            indices: path.match_indices('/'),
        }
    }

    pub fn lookahead(self) -> SplitWithLookahead<'a> {
        SplitWithLookahead {
            split: self.peekable(),
        }
    }
}

impl Iterator for Split<'_> {
    type Item = [usize; 2];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((end, _)) = self.indices.next() {
            if end == 0 {
                self.offset += 1;
                continue;
            }

            return Some([mem::replace(&mut self.offset, end + 1), end]);
        }

        if &self.len == &self.offset {
            None
        } else {
            Some([mem::replace(&mut self.offset, self.len), self.len])
        }
    }
}

impl SplitWithLookahead<'_> {
    pub fn has_next(&mut self) -> bool {
        self.split.peek().is_some()
    }
}

impl Iterator for SplitWithLookahead<'_> {
    type Item = (bool, [usize; 2]);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.split.next()?;
        Some((self.split.peek().is_none(), next))
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
            vec![[1, 1], [2, 6], [7, 7], [8, 13]],
            vec![[1, 9], [10, 10], [11, 15], [16, 19]],
            vec![[1, 5], [6, 11], [12, 16], [17, 17], [18, 22]],
            vec![[1, 5], [6, 6], [7, 14], [15, 23]],
            vec![[1, 9], [10, 17], [18, 18], [19, 21]],
            vec![],
        ]
    }

    #[test]
    fn test_split_into() {
        let expected_results = get_expected_results();

        for (i, path) in PATHS.iter().enumerate() {
            assert_eq!(
                Split::new(path).count(),
                expected_results[i].len(),
                "Split produced more or less segments than expected for {}",
                path
            );

            for (j, segment) in Split::new(path).enumerate() {
                let expect = expected_results[i][j];
                assert_eq!(segment, expect, "{} ({}, {:?})", path, j, segment);
            }
        }
    }
}
