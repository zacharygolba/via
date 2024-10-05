use core::iter::Enumerate;
use core::str::Bytes;

/// A matched range in the url path.
///
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
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

impl Span {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.start
    }
}

impl Span {
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
    type Item = Span;

    fn next(&mut self) -> Option<Self::Item> {
        // Set the start index to the next character that is not a `/`.
        let start = self.next_non_terminator()?;
        // Set the end index to the next character that is a `/`.
        let end = self.next_terminator().unwrap_or(self.value.len());

        // Return the start and end offset of the current path segment.
        Some(Span { start, end })
    }
}

#[cfg(test)]
mod tests {
    use super::{Span, SplitPath};

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
    fn test_split_path() {
        let expected_results = get_expected_results();

        for (path_index, path_value) in PATHS.iter().enumerate() {
            for (segment_index, segment_value) in SplitPath::new(path_value).enumerate() {
                assert_eq!(segment_value, expected_results[path_index][segment_index]);
            }
        }
    }
}
