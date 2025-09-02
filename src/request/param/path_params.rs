use std::fmt::{self, Debug, Formatter};
use std::slice;

#[derive(Default)]
pub struct PathParams {
    data: Vec<(String, [usize; 2])>,
}

impl PathParams {
    #[inline]
    pub fn new(data: Vec<(String, [usize; 2])>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, (String, [usize; 2])> {
        self.data.iter()
    }
}

impl Extend<(String, [usize; 2])> for PathParams {
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, [usize; 2])>,
    {
        self.data.extend(iter);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
