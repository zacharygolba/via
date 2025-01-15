#[derive(PartialEq)]
pub enum Pattern {
    Root,
    Static(String),
    Dynamic(String),
    Wildcard(String),
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
            Some(':') => match segment.get(1..) {
                None | Some("") => panic!("Dynamic parameters must be named. Found ':'."),
                Some(name) => Pattern::Dynamic(name.into()),
            },

            // Path segments that start with an asterisk are considered CatchAll
            // pattern. The remaining characters in the segment are considered
            // the name or identifier associated with the parameter.
            Some('*') => match segment.get(1..) {
                None | Some("") => panic!("Wildcard parameters must be named. Found '*'."),
                Some(name) => Pattern::Wildcard(name.into()),
            },

            // The segment does not start with a reserved character. We will
            // consider it a static pattern that can be matched by value.
            _ => Pattern::Static(segment.into()),
        }
    })
}
