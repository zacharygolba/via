use std::fmt::{self, Debug, Formatter};
use std::slice::Iter;

pub struct PathParams {
    data: Vec<(String, (usize, usize))>,
}

impl PathParams {
    #[inline]
    pub fn new(data: Vec<(String, (usize, usize))>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> Iter<(String, (usize, usize))> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (String, (usize, usize))) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
