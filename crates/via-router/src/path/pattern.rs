use std::fmt::{self, Debug, Display};
use std::sync::Arc;

use super::{Span, SplitPath};

#[derive(PartialEq)]
pub enum Pattern {
    Root,
    Static(Param),
    Dynamic(Param),
    CatchAll(Param),
}

/// An identifier for a named path segment.
///
#[derive(Debug, PartialEq)]
pub struct Param {
    ident: Arc<str>,
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    SplitPath::new(path).map(|span| {
        let Span { start, end } = span;
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
    pub fn as_str(&self) -> &str {
        &self.ident
    }
}

impl Param {
    pub(crate) fn new(ident: &str) -> Self {
        Self {
            ident: ident.into(),
        }
    }
}

impl Clone for Param {
    fn clone(&self) -> Self {
        Self {
            ident: Arc::clone(&self.ident),
        }
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ident, f)
    }
}
