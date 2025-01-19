#![allow(clippy::type_complexity)]

use std::fmt::{self, Debug, Formatter};
use std::slice;
use tinyvec::TinyVec;
use via_router::Param;

pub struct PathParams {
    data: TinyVec<[(Param, (usize, usize)); 1]>,
}

impl PathParams {
    #[inline]
    pub fn new(data: TinyVec<[(Param, (usize, usize)); 1]>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(Param, (usize, usize))> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (Param, (usize, usize))) {
        if self.data.len() == 1 {
            self.data.reserve(7);
        }

        self.data.push(param);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
