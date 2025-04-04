use std::fmt::{self, Debug, Formatter};
use std::slice;
use via_router::Param;

#[derive(Default)]
pub struct PathParams {
    data: Vec<(Param, [usize; 2])>,
}

impl PathParams {
    #[inline]
    pub fn new(data: Vec<(Param, [usize; 2])>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(Param, [usize; 2])> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, name: Param, range: [usize; 2]) {
        self.data.push((name, range));
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
