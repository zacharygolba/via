use core::iter::Enumerate;
use core::str::Bytes;

/// A matched range in the url path.
///
#[derive(Clone, Debug, PartialEq)]
pub struct SegmentAt {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// An iterator that yields a tuple containing the start and end offset of each
/// segment in the url path.
///
pub struct SplitPath<'a> {
    bytes: Enumerate<Bytes<'a>>,
    value: &'a str,
}

impl SegmentAt {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

impl SegmentAt {
    pub(crate) fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl<'a> SplitPath<'a> {
    pub fn new(value: &'a str) -> Self {
        SplitPath {
            bytes: value.bytes().enumerate(),
            value,
        }
    }
}

impl<'a> SplitPath<'a> {
    /// Advances `self.iter` to the next occurance of a character that is not a
    /// `/` character and returns the index.
    fn next_non_terminator(&mut self) -> Option<usize> {
        self.bytes.find_map(|(index, byte)| {
            if byte != b'/' && self.value.is_char_boundary(index) {
                // We found a character that is not a `/`. Return the index.
                Some(index)
            } else {
                // Skip the character and continue searching for the next
                // non-terminator character.
                None
            }
        })
    }

    /// Advances `self.iter` to the next occurance of a `/` character and
    /// returns the index.
    fn next_terminator(&mut self) -> Option<usize> {
        self.bytes.find_map(|(index, byte)| {
            if byte == b'/' && self.value.is_char_boundary(index) {
                // We found a character that is a `/`. Return the index.
                Some(index)
            } else {
                // Skip the character and continue searching for the next
                // terminator character.
                None
            }
        })
    }
}

impl<'a> Iterator for SplitPath<'a> {
    type Item = SegmentAt;

    fn next(&mut self) -> Option<Self::Item> {
        // Set the start index to the next character that is not a `/`.
        let start = self.next_non_terminator()?;
        // Set the end index to the next character that is a `/`.
        let end = self.next_terminator().unwrap_or(self.value.len());

        // Return the start and end offset of the current path segment.
        Some(SegmentAt { start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::{SegmentAt, SplitPath};

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

    fn get_expected_results() -> [Vec<SegmentAt>; 15] {
        [
            vec![SegmentAt::new(1, 5), SegmentAt::new(6, 11)],
            vec![
                SegmentAt::new(1, 9),
                SegmentAt::new(10, 14),
                SegmentAt::new(15, 18),
            ],
            vec![
                SegmentAt::new(1, 5),
                SegmentAt::new(6, 11),
                SegmentAt::new(12, 16),
                SegmentAt::new(17, 21),
            ],
            vec![
                SegmentAt::new(1, 5),
                SegmentAt::new(6, 13),
                SegmentAt::new(14, 22),
            ],
            vec![SegmentAt::new(1, 9), SegmentAt::new(10, 17)],
            vec![SegmentAt::new(1, 7), SegmentAt::new(8, 23)],
            vec![SegmentAt::new(1, 5), SegmentAt::new(6, 12)],
            vec![SegmentAt::new(1, 10), SegmentAt::new(11, 18)],
            vec![SegmentAt::new(1, 4)],
            vec![SegmentAt::new(1, 8), SegmentAt::new(9, 16)],
            vec![SegmentAt::new(2, 6), SegmentAt::new(8, 13)],
            vec![
                SegmentAt::new(1, 9),
                SegmentAt::new(11, 15),
                SegmentAt::new(16, 19),
            ],
            vec![
                SegmentAt::new(1, 5),
                SegmentAt::new(6, 11),
                SegmentAt::new(12, 16),
                SegmentAt::new(18, 22),
            ],
            vec![
                SegmentAt::new(1, 5),
                SegmentAt::new(7, 14),
                SegmentAt::new(15, 23),
            ],
            vec![
                SegmentAt::new(1, 9),
                SegmentAt::new(10, 17),
                SegmentAt::new(19, 21),
            ],
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
