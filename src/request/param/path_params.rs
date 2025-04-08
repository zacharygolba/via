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
}

impl Extend<(Param, [usize; 2])> for PathParams {
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (Param, [usize; 2])>,
    {
        self.data.extend(iter);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
