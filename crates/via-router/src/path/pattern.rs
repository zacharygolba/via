use core::fmt::{self, Display};

use super::SplitPath;

#[derive(Debug, PartialEq)]
pub enum Pattern {
    Root,
    Static(Param),
    Dynamic(Param),
    CatchAll(Param),
}

/// An identifier for a named path segment.
///
#[derive(Copy, Debug, PartialEq)]
pub struct Param {
    ident: &'static str,
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    SplitPath::new(path).map(|(start, end)| {
        let segment = path.get(start..end).unwrap_or("");

        match segment.chars().next() {
            Some(':') => {
                let rest = segment.get(1..).unwrap_or("");
                Pattern::Dynamic(Param::new(rest))
            }
            Some('*') => {
                let rest = segment.get(1..).unwrap_or("");
                Pattern::CatchAll(Param::new(rest))
            }
            _ => {
                // Segment does not contain a dynamic parameter.
                Pattern::Static(Param::new(segment))
            }
        }
    })
}

impl Param {
    pub fn as_str(&self) -> &'static str {
        self.ident
    }
}

impl Param {
    pub(crate) fn new(ident: &'static str) -> Self {
        Self { ident }
    }
}

impl Clone for Param {
    fn clone(&self) -> Self {
        Self { ident: self.ident }
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ident, f)
    }
}

impl PartialEq<str> for Param {
    fn eq(&self, other: &str) -> bool {
        self.ident == other
    }
}
