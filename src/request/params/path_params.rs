use std::fmt::{self, Debug, Formatter};
use via_router::Param;

pub struct PathParams {
    data: Vec<(Param, usize, usize)>,
}

impl PathParams {
    pub fn new(data: Vec<(Param, usize, usize)>) -> Self {
        Self { data }
    }

    pub fn get(&self, predicate: &str) -> Option<(usize, usize)> {
        self.data.iter().find_map(|(name, start, end)| {
            if predicate == name.as_str() {
                Some((*start, *end))
            } else {
                None
            }
        })
    }

    pub fn push(&mut self, param: (Param, usize, usize)) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
