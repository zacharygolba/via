#![allow(clippy::type_complexity)]

use std::fmt::{self, Debug, Formatter};
use std::slice;
use std::sync::Arc;
use tinyvec::TinyVec;

pub struct PathParams {
    data: TinyVec<[(Arc<str>, (usize, usize)); 1]>,
}

impl PathParams {
    #[inline]
    pub fn new(data: TinyVec<[(Arc<str>, (usize, usize)); 1]>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(Arc<str>, (usize, usize))> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (Arc<str>, (usize, usize))) {
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
