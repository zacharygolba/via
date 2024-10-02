use std::fmt::{self, Debug, Formatter};
use via_router::Param;

pub struct PathParams {
    data: Vec<(Param, [usize; 2])>,
}

impl PathParams {
    pub fn new(data: Vec<(Param, [usize; 2])>) -> Self {
        Self { data }
    }

    pub fn get(&self, predicate: &str) -> Option<&[usize; 2]> {
        self.data.iter().find_map(|(name, at)| {
            if predicate == name.as_str() {
                Some(at)
            } else {
                None
            }
        })
    }

    pub fn push(&mut self, param: (Param, [usize; 2])) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
