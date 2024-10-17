use std::error::Error;

/// An iterator over the sources of an `Error`.
///
#[derive(Debug)]
pub struct Iter<'a> {
    source: Option<&'a dyn Error>,
}

impl<'a> Iter<'a> {
    pub fn new(source: Option<&'a dyn Error>) -> Self {
        Self { source }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a dyn Error;

    fn next(&mut self) -> Option<Self::Item> {
        // Attempt to get a copy of the source error from self. If the source
        // field is None, return early.
        let next = self.source?;

        // Set self.source to the next source error.
        self.source = next.source();

        // Return the next error.
        Some(next)
    }
}
