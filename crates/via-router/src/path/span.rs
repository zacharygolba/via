use smallvec::SmallVec;

/// A matched range in the url path.
///
#[derive(Debug, PartialEq)]
pub struct Span {
    start: usize,
    end: usize,
}

pub fn split(path: &'static str) -> Vec<Span> {
    let mut segments = Vec::new();
    let mut start = None;

    for (index, byte) in path.bytes().enumerate() {
        match (start, byte) {
            // The start index is set and the current byte is a `/`. This is the
            // end of the current segment.
            (Some(from), b'/') => {
                // Push the segment range to the segments vector.
                segments.push(Span::new(from, index));

                // Reset the start index.
                start = None;
            }

            // The start index is set and the current byte is not a terminating
            // `/`. Continue to the next byte.
            (Some(_), _) => {}

            // The start index is not set and the current byte is a `/`. Continue
            // to the next byte.
            (None, b'/') => {}

            // The current byte is a `/` and the start index is not set. This is
            // the start of a new segment.
            (None, _) => {
                // Set the start index to the current index.
                start = Some(index);
            }
        }
    }

    // It is likely that the last character in the path is not a `/`. Check if
    // the start index is set. If so, push the last segment to the segments vec
    // using the length of the path as the end index.
    if let Some(from) = start {
        segments.push(Span::new(from, path.len()));
    }

    segments
}

#[inline]
pub fn split_into(segments: &mut SmallVec<[Span; 5]>, path: &str) {
    let mut start = None;

    for (index, byte) in path.chars().enumerate() {
        match (start, byte) {
            // The start index is set and the current byte is a `/`. This is the
            // end of the current segment.
            (Some(from), '/') => {
                // Push the segment range to the segments vector.
                segments.push(Span::new(from, index));

                // Reset the start index.
                start = None;
            }

            // The start index is set and the current byte is not a terminating
            // `/`. Continue to the next byte.
            (Some(_), _) => {}

            // The start index is not set and the current byte is a `/`. Continue
            // to the next byte.
            (None, '/') => {}

            // The current byte is a `/` and the start index is not set. This is
            // the start of a new segment.
            (None, _) => {
                // Set the start index to the current index.
                start = Some(index);
            }
        }
    }

    // It is likely that the last character in the path is not a `/`. Check if
    // the start index is set. If so, push the last segment to the segments vec
    // using the length of the path as the end index.
    if let Some(from) = start {
        segments.push(Span::new(from, path.len()));
    }
}

impl Span {
    /// Returns the start offset of the matched range.
    ///
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the end offset of the matched range.
    ///
    pub fn end(&self) -> usize {
        self.end
    }
}

impl Span {
    pub(crate) fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl Clone for Span {
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

    use super::{split, split_into, Span};

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

    fn get_expected_results() -> [Vec<Span>; 15] {
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
        ]
    }

    #[test]
    fn test_split() {
        let expected_results = get_expected_results();

        for (path_index, path_value) in PATHS.iter().enumerate() {
            let segments = split(path_value);

            for (segment_index, segment_value) in segments.into_iter().enumerate() {
                assert_eq!(segment_value, expected_results[path_index][segment_index]);
            }
        }
    }

    #[test]
    fn test_split_into() {
        let expected_results = get_expected_results();

        for (path_index, path_value) in PATHS.iter().enumerate() {
            let mut segments = SmallVec::new();

            split_into(&mut segments, path_value);

            for (segment_index, segment_value) in segments.into_iter().enumerate() {
                assert_eq!(segment_value, expected_results[path_index][segment_index]);
            }
        }
    }
}
