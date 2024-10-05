use std::fmt::{self, Debug, Formatter};
use via_router::{Param, SegmentAt};

pub struct PathParams {
    data: Vec<(Param, SegmentAt)>,
}

impl PathParams {
    pub fn new(data: Vec<(Param, SegmentAt)>) -> Self {
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

    pub fn push(&mut self, param: (Param, SegmentAt)) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
