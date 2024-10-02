use std::fmt::{self, Debug, Display};

use super::SplitPath;

#[derive(PartialEq)]
pub enum Pattern {
    Root,
    Static(ParamName),
    Dynamic(ParamName),
    CatchAll(ParamName),
}

/// An identifier for a named path segment.
///
#[derive(Debug, PartialEq)]
pub struct ParamName {
    ident: Box<str>,
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    SplitPath::new(path).map(|[start, end]| {
        let segment = path.get(start..end).unwrap_or("");

        match segment.chars().next() {
            Some(':') => {
                let rest = segment.get(1..).unwrap_or("");
                Pattern::Dynamic(ParamName::new(rest))
            }
            Some('*') => {
                let rest = segment.get(1..).unwrap_or("");
                Pattern::CatchAll(ParamName::new(rest))
            }
            _ => {
                // Segment does not contain a dynamic parameter.
                Pattern::Static(ParamName::new(segment))
            }
        }
    })
}

impl ParamName {
    pub fn as_str(&self) -> &str {
        &self.ident
    }
}

impl ParamName {
    pub(crate) fn new(ident: &str) -> Self {
        Self {
            ident: ident.into(),
        }
    }
}

impl Clone for ParamName {
    fn clone(&self) -> Self {
        Self {
            ident: self.ident.clone(),
        }
    }
}

impl Display for ParamName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ident, f)
    }
}
