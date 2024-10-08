use std::fmt::{self, Debug, Display};

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
    ident: Box<str>,
}

/// Unwraps the remaining slice of a path segment after the first char or
/// panics with a custom message.
///
macro_rules! rest_or {
    (
        // Should evaluate to a `&str`.
        $segment:expr,
        // Args passed to panic! if the segment is empty.
        $($arg:tt)*
    ) => {{
        let segment = $segment;
        match segment.get(1..) {
            // The range is out of bounds or produced an empty str.
            None | Some("") => panic!($($arg)*),
            // The range is valid.
            Some(rest) => rest,
        }
    }};
}

/// Returns an iterator that yields a `Pattern` for each segment in the uri path.
///
pub fn patterns(path: &'static str) -> impl Iterator<Item = Pattern> {
    super::split(path).into_iter().map(|at| {
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
                let rest = rest_or!(segment, "Dynamic parameters must be named. Found ':'.");
                let param = Param::new(rest);

                Pattern::Dynamic(param)
            }

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => {
                let rest = rest_or!(segment, "Wildcard parameters must be named. Found '*'.");
                let param = Param::new(rest);

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
            ident: self.ident.clone(),
        }
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.ident, f)
    }
}
