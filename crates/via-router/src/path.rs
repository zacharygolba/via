use std::str::CharIndices;

#[derive(Clone, PartialEq)]
pub enum Pattern {
    CatchAll(&'static str),
    Dynamic(&'static str),
    Static(&'static str),
    Root,
}

/// Represents a url path with start and indices of each segment in the url
/// path separated by `/`.
pub struct PathSegments<'a> {
    pub value: &'a str,
    segments: Vec<(usize, usize)>,
}

/// An iterator that splits the path into segments and yields a key-value pair
/// containing the start and end offset of the substring separated by `/`.
#[derive(Debug)]
pub struct SplitPath<'a> {
    len: usize,
    iter: CharIndices<'a>,
    offset: usize,
}

/// Returns an iterator that yields a `Pattern` for each segment in the url path.
pub fn patterns(value: &'static str) -> impl Iterator<Item = Pattern> {
    SplitPath::new(value).map(|(start, end)| Pattern::from(&value[start..end]))
}

/// Returns an iterator that yields a key-value pair containing the start and
/// end offset of each segment in the url path.
pub fn segments(value: &str) -> PathSegments {
    let mut segments = Vec::with_capacity(10);

    segments.extend(SplitPath::new(value));
    PathSegments { value, segments }
}

impl From<&'static str> for Pattern {
    fn from(value: &'static str) -> Pattern {
        match value.chars().next() {
            Some('*') => Self::CatchAll(&value[1..]),
            Some(':') => Self::Dynamic(&value[1..]),
            _ => Self::Static(value),
        }
    }
}

impl PartialEq<&str> for Pattern {
    fn eq(&self, other: &&str) -> bool {
        if let Self::Static(value) = *self {
            value == *other
        } else {
            true
        }
    }
}

impl PartialEq<Pattern> for &str {
    fn eq(&self, other: &Pattern) -> bool {
        other == self
    }
}

impl<'a> PathSegments<'a> {
    /// Returns the value of the current path segment that we are attempting to
    /// match if it exists. The returned value should only be `None` if we are
    /// attempting to match a root url path (i.e `"/"`).
    pub fn get(&self, index: usize) -> Option<&(usize, usize)> {
        self.segments.get(index)
    }

    /// Returns a key value pair containing the start offset of the path segment
    /// at `index` and the end offset of the last path segment in the url path.
    ///
    /// This is used to get the start and end offset of a `CatchAll` pattern.
    pub fn slice_from(&self, index: usize) -> (usize, usize) {
        self.segments
            .get(index)
            .zip(self.segments.last())
            .map_or((0, 0), |((start, _), (_, end))| (*start, *end))
    }
}

impl<'a> SplitPath<'a> {
    pub fn new(value: &'a str) -> Self {
        Self {
            len: value.len(),
            iter: value.char_indices(),
            offset: 0,
        }
    }
}

impl<'a> Iterator for SplitPath<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = self.offset;
        let mut end = self.len;

        while let (index, '/') = self.iter.next()? {
            start = index + 1;
        }

        for (index, char) in &mut self.iter {
            if char == '/' {
                end = index;
                break;
            }
        }

        self.offset = end + 1;

        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::SplitPath;

    const PATHS: [&str; 15] = [
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
    ];

    fn get_expected_results() -> [Vec<(usize, usize)>; 15] {
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
        ]
    }

    #[test]
    fn test_split_path() {
        let expected_results = get_expected_results();

        for (path_index, path_value) in PATHS.iter().enumerate() {
            for (segment_index, segment_value) in SplitPath::new(path_value).enumerate() {
                assert_eq!(segment_value, expected_results[path_index][segment_index]);
            }
        }
    }
}
