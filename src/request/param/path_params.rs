use std::slice;
use std::sync::Arc;
use via_router::Param;

#[derive(Debug)]
pub struct PathParams(Vec<Param>);

impl PathParams {
    #[inline]
    pub fn new(data: Vec<Param>) -> Self {
        Self(data)
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, Param> {
        self.0.iter()
    }
}

impl PathParams {
    #[inline]
    pub(crate) fn push(&mut self, name: Arc<str>, range: (usize, Option<usize>)) {
        self.0.push((name, range));
    }
}
