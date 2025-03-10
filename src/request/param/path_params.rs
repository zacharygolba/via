use std::fmt::{self, Debug, Formatter};
use std::slice;
use via_router::Param;

pub struct PathParams {
    data: Vec<(Param, Option<[usize; 2]>)>,
}

impl PathParams {
    #[inline]
    pub fn new(data: Vec<(Param, Option<[usize; 2]>)>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<(Param, Option<[usize; 2]>)> {
        self.data.iter()
    }

    #[inline]
    pub fn push(&mut self, name: &Param, range: Option<[usize; 2]>) {
        self.data.push((name.clone(), range));
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
