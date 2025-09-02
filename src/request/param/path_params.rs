use std::fmt::{self, Debug, Formatter};
use std::slice;

#[derive(Default)]
pub struct PathParams {
    data: Vec<(String, (usize, Option<usize>))>,
}

impl PathParams {
    #[inline]
    pub fn new(data: Vec<(String, (usize, Option<usize>))>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, (String, (usize, Option<usize>))> {
        self.data.iter()
    }
}

impl Extend<(String, (usize, Option<usize>))> for PathParams {
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (String, (usize, Option<usize>))>,
    {
        self.data.extend(iter);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
