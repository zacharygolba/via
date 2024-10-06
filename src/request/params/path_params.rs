use std::fmt::{self, Debug, Formatter};
use via_router::{Param, Span};

pub struct PathParams {
    data: Vec<(Param, Span)>,
}

impl PathParams {
    pub fn new(data: Vec<(Param, Span)>) -> Self {
        Self { data }
    }

    pub fn get(&self, predicate: &str) -> Option<(usize, usize)> {
        self.data.iter().find_map(|(name, at)| {
            if predicate == name.as_str() {
                Some((at.start(), at.end()))
            } else {
                None
            }
        })
    }

    pub fn push(&mut self, param: (Param, Span)) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
