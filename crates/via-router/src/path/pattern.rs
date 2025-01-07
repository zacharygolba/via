use std::fmt::{self, Debug, Display};
use std::sync::Arc;

#[derive(PartialEq)]
pub enum Pattern {
    Root,
    Static(Param),
    Dynamic(Param),
    Wildcard(Param),
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
    let mut segments = vec![];

    super::split(&mut segments, path);
    segments.into_iter().map(|at| {
        let end = at.end();
        let start = at.start();
        let segment = match path.get(start..end) {
            Some(slice) => slice,
            None => panic!("Path segments cannot be empty when defining a route."),
        };

        match segment.chars().next() {
            // Path segments that start with a colon are considered a Dynamic
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some(':') => {
                let param = Param::new(match segment.get(1..) {
                    None | Some("") => panic!("Dynamic parameters must be named. Found ':'."),
                    Some(rest) => rest,
                });

                Pattern::Dynamic(param)
            }

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => {
                let param = Param::new(match segment.get(1..) {
                    None | Some("") => panic!("Wildcard parameters must be named. Found '*'."),
                    Some(rest) => rest,
                });

                Pattern::Wildcard(param)
            }

            // The segment does not start with a reserved character. We will
            // consider it a static pattern that can be matched by value.
            _ => {
                let param = Param::new(segment);
                Pattern::Static(param)
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
