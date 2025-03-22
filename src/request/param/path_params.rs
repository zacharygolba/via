use std::fmt::{self, Debug, Formatter};
use std::slice;
use via_router::Param;

#[derive(Default)]
pub struct PathParams {
    data: Vec<(Param, Option<[usize; 2]>)>,
}

impl PathParams {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(Param, Option<[usize; 2]>)> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (Param, Option<[usize; 2]>)) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
