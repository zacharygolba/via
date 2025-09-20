use std::sync::Arc;

pub type Param = (usize, Option<usize>);

#[derive(Clone)]
pub struct Split<'a> {
    path: &'a str,
    offset: usize,
}

#[derive(Debug, PartialEq)]
pub enum Pattern {
    Root,
    Static(String),
    Dynamic(Arc<str>),
    Wildcard(Arc<str>),
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub(crate) fn patterns(path: &str) -> impl Iterator<Item = Pattern> + '_ {
    Split::new(path).map(|(segment, _)| {
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

impl<'a> Split<'a> {
    #[inline]
    pub fn new(path: &'a str) -> Self {
        Self {
            path,
            offset: if path.starts_with('/') { 1 } else { 0 },
        }
    }
}

impl<'a> Iterator for Split<'a> {
    type Item = (&'a str, [usize; 2]);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let offset = &mut self.offset;
        let start = *offset;
        let path = self.path;

        match path.get(start..).and_then(|rest| {
            rest.bytes()
                .enumerate()
                .find_map(|(index, byte)| (byte == b'/').then_some(index))
        }) {
            Some(len) => {
                let end = start + len;
                *offset = end + 1;
                Some((&path[start..end], [start, end]))
            }
            None => {
                let end = path.len();
                *offset = end;

                // Only yield if there's something left between offset and path.len().
                // Prevents slicing past the end on trailing slashes like "/via/".
                if end > start {
                    Some((&path[start..end], [start, end]))
                } else {
                    None
                }
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
                let [start, end] = expected_results[i][j];
                let expect = (&path[start..end], [start, end]);

                assert_eq!(segment, expect, "{} ({}, {:?})", path, j, segment);
            }
        }
    }
}
