use smallvec::SmallVec;
use std::{iter::Peekable, str::CharIndices};

use crate::node::Pattern;

/// Represents a url path with start and indices of each segment in the url
/// path separated by `/`.
pub(crate) struct PathSegments<'a> {
    pub(crate) value: &'a str,
    segments: SmallVec<[(usize, usize); 6]>,
}

/// An iterator that splits the path into segments and yields a key-value pair
/// containing the start and end offset of the substring separated by `/`.
#[derive(Debug, Clone)]
pub(crate) struct SplitPath<'a> {
    chars: Peekable<CharIndices<'a>>,
    value: &'a str,
}

impl<'a> PathSegments<'a> {
    pub(crate) fn new(value: &'a str) -> Self {
        PathSegments {
            value,
            segments: SplitPath::new(value).collect(),
        }
    }

    /// Returns the value of the current path segment that we are attempting to
    /// match if it exists. The returned value should only be `None` if we are
    /// attempting to match a root url path (i.e `"/"`).
    pub(crate) fn get(&self, index: usize) -> Option<(usize, usize)> {
        self.segments.get(index).copied()
    }

    /// Return `true` if the segment located at `index` is the last segment in
    /// the url path.
    pub(crate) fn is_last_segment(&self, index: usize) -> bool {
        index == self.segments.len() - 1
    }

    /// Returns a key value pair containing the start offset of the path segment
    /// at `index` and the end offset of the last path segment in the url path.
    ///
    /// This is used to get the start and end offset of a `CatchAll` pattern.
    pub(crate) fn slice_from(&self, index: usize) -> (usize, usize) {
        self.segments
            .get(index)
            .map(|(start, _)| *start)
            .zip(self.segments.last().map(|(_, end)| *end))
            .unwrap_or((0, 0))
    }
}

impl<'a> SplitPath<'a> {
    pub(crate) fn new(value: &'a str) -> Self {
        SplitPath {
            chars: value.char_indices().peekable(),
            value,
        }
    }
}

impl SplitPath<'static> {
    pub(crate) fn into_patterns(self) -> impl Iterator<Item = Pattern> {
        let value = self.value;
        self.map(|(start, end)| Pattern::from(&value[start..end]))
    }
}

impl<'a> Iterator for SplitPath<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let mut start = None;
        let mut end = self.value.len();

        while let (index, '/') = *self.chars.peek()? {
            start = Some(index + 1);
            self.chars.next();
        }

        while let Some((index, value)) = self.chars.peek() {
            if *value == '/' {
                end = *index;
                break;
            }

            self.chars.next();
        }

        Some((start?, end))
    }
}
