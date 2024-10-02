use core::iter::Enumerate;
use core::str::Bytes;

/// An iterator that yields a tuple containing the start and end offset of each
/// segment in the url path.
///
pub struct SplitPath<'a> {
    bytes: Enumerate<Bytes<'a>>,
    value: &'a str,
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
    type Item = [usize; 2];

    fn next(&mut self) -> Option<Self::Item> {
        // Set the start index to the next character that is not a `/`.
        let start = self.next_non_terminator()?;
        // Set the end index to the next character that is a `/`.
        let end = self.next_terminator().unwrap_or(self.value.len());

        // Return the start and end offset of the current path segment.
        Some([start, end])
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

    fn get_expected_results() -> [Vec<[usize; 2]>; 15] {
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
