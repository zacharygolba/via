use std::fmt::{self, Debug, Formatter};
use std::slice;
use via_router::Param;

#[derive(Default)]
pub struct PathParams {
    data: Vec<Param>,
}

impl PathParams {
    #[inline]
    pub fn new(data: Vec<Param>) -> Self {
        Self { data }
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, Param> {
        self.data.iter()
    }
}

impl Extend<Param> for PathParams {
    #[inline]
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Param>,
    {
        self.data.extend(iter);
    }
}

impl Debug for PathParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.data, f)
    }
}
