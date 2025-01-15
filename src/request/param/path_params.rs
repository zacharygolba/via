use std::fmt::{self, Debug, Formatter};
use std::slice::Iter;
use via_router::Span;

pub struct PathParams {
    data: Vec<(String, Span)>,
}

impl PathParams {
    #[inline]
    pub fn new() -> Self {
        Self {
            data: Default::default(),
        }
    }

    #[inline]
    pub fn iter(&self) -> Iter<(String, Span)> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (String, Span)) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
