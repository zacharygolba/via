use smallvec::SmallVec;

/// Defines the behavior of a collection that can have elements appended
/// to the end.
///
pub trait Push {
    fn push(&mut self, span: Span);
}

/// A matched range in the url path.
///
#[derive(Debug, PartialEq)]
pub struct Span {
    start: usize,
    end: usize,
}

pub fn split_into(segments: &mut impl Push, path: &str) {
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
            segments.push(Span::new(start, end));
        }

        // Move start to the byte position after end.
        start = end + 1;
    }

    // If a range exists with a length greater than `0` from start to the
    // length of path, append a Span to segments. This condition is true
    // when path does not end with `/`.
    if len > start {
        segments.push(Span::new(start, len));
    }
}

impl Push for SmallVec<[Span; 5]> {
    #[inline]
    fn push(&mut self, span: Span) {
        SmallVec::push(self, span);
    }
}

impl Push for Vec<Span> {
    #[inline]
    fn push(&mut self, span: Span) {
        Vec::push(self, span);
    }
}

impl Span {
    /// Returns the start offset of the matched range.
    ///
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end offset of the matched range.
    ///
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
}

impl Span {
    #[inline]
    pub(crate) fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl Clone for Span {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use smallvec::SmallVec;

    use super::{split_into, Span};

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

    fn get_expected_results() -> [Vec<Span>; 16] {
        [
            vec![Span::new(1, 5), Span::new(6, 11)],
            vec![Span::new(1, 9), Span::new(10, 14), Span::new(15, 18)],
            vec![
                Span::new(1, 5),
                Span::new(6, 11),
                Span::new(12, 16),
                Span::new(17, 21),
            ],
            vec![Span::new(1, 5), Span::new(6, 13), Span::new(14, 22)],
            vec![Span::new(1, 9), Span::new(10, 17)],
            vec![Span::new(1, 7), Span::new(8, 23)],
            vec![Span::new(1, 5), Span::new(6, 12)],
            vec![Span::new(1, 10), Span::new(11, 18)],
            vec![Span::new(1, 4)],
            vec![Span::new(1, 8), Span::new(9, 16)],
            vec![Span::new(2, 6), Span::new(8, 13)],
            vec![Span::new(1, 9), Span::new(11, 15), Span::new(16, 19)],
            vec![
                Span::new(1, 5),
                Span::new(6, 11),
                Span::new(12, 16),
                Span::new(18, 22),
            ],
            vec![Span::new(1, 5), Span::new(7, 14), Span::new(15, 23)],
            vec![Span::new(1, 9), Span::new(10, 17), Span::new(19, 21)],
            vec![],
        ]
    }

    #[test]
    fn test_split_into() {
        let expected_results = get_expected_results();

        for (path_index, path_value) in PATHS.iter().enumerate() {
            let mut segments = SmallVec::new();

            split_into(&mut segments, path_value);

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
