#![allow(clippy::type_complexity)]

use std::fmt::{self, Debug, Formatter};
use std::slice;
use via_router::Param;

pub struct PathParams {
    data: Vec<(Param, (usize, usize))>,
}

impl PathParams {
    #[inline]
    pub const fn new(data: Vec<(Param, (usize, usize))>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(Param, (usize, usize))> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (Param, (usize, usize))) {
        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
