use std::fmt::{self, Debug, Display};

use super::SplitPath;

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
    ident: String,
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    SplitPath::new(path).map(|at| {
        let end = at.end();
        let start = at.start();
        let segment = path.get(start..end).unwrap_or_default();

        match segment.chars().next() {
            // Path segments that start with a colon are considered a Dynamic
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some(':') => Pattern::Dynamic(Param::new(&segment[1..])),

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => Pattern::CatchAll(Param::new(&segment[1..])),

            // The segment does not start with a reserved character. We'll
            // consider it a static pattern and match it against uri path
            // segments by value.
            _ => Pattern::Static(Param::new(segment)),
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
            ident: ident.to_owned(),
        }
    }
}

impl Clone for Param {
    fn clone(&self) -> Self {
        Self {
            ident: self.ident.clone(),
        }
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ident, f)
    }
}
