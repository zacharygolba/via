use std::str::CharIndices;

#[derive(PartialEq)]
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
struct SplitPath<'a> {
    len: usize,
    iter: CharIndices<'a>,
    offset: usize,
}

/// Returns an iterator that yields a `Pattern` for each segment in the url path.
pub fn patterns(value: &'static str) -> impl Iterator<Item = Pattern> {
    split(value).map(|(start, end)| Pattern::from(&value[start..end]))
}

/// Returns a collection containing the start and end offset of each segment in
/// the url path.
pub fn segments(value: &str) -> PathSegments {
    let mut segments = Vec::with_capacity(10);

    segments.extend(split(value));
    PathSegments { value, segments }
}

/// Returns an iterator that yields a tuple containing the start and end offset
/// of each segment in the url path.
fn split(value: &str) -> SplitPath {
    SplitPath {
        len: value.len(),
        iter: value.char_indices(),
        offset: 0,
    }
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

impl<'a> PathSegments<'a> {
    /// Returns the value of the current path segment that we are attempting to
    /// match if it exists. The returned value should only be `None` if we are
    /// attempting to match a root url path (i.e `"/"`).
    pub fn get(&self, index: usize) -> Option<&(usize, usize)> {
        self.segments.get(index)
    }

    /// Returns the number of segments in the url path.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Returns `true` if the url path has no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
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

impl<'a> Iterator for SplitPath<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = self.offset;
        let mut end = self.len;

        // Advance the start index to the next character that is not a `/`.
        while let (index, '/') = self.iter.next()? {
            start = index + 1;
        }

        // Advance the end index to the next character that is a `/`.
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
    use super::split;

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
            for (segment_index, segment_value) in split(path_value).enumerate() {
                assert_eq!(segment_value, expected_results[path_index][segment_index]);
            }
        }
    }
}
