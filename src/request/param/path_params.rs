use std::fmt::{self, Debug, Formatter};
use std::slice;
use tinyvec::TinyVec;

pub struct PathParams {
    data: TinyVec<[(String, (usize, usize)); 1]>,
}

impl PathParams {
    #[inline]
    pub fn new(data: TinyVec<[(String, (usize, usize)); 1]>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(String, (usize, usize))> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, param: (String, (usize, usize))) {
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
