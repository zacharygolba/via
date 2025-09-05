use std::slice;
use std::sync::Arc;
use via_router::Param;

#[derive(Debug)]
pub struct PathParams(Vec<(Arc<str>, Param)>);

impl PathParams {
    #[inline]
    pub fn new(data: Vec<(Arc<str>, Param)>) -> Self {
        Self(data)
    }

    #[inline]
    pub fn iter(&self) -> slice::Iter<'_, (Arc<str>, Param)> {
        self.0.iter()
    }
}

impl PathParams {
    #[inline]
    pub(crate) fn push(&mut self, name: Arc<str>, range: (usize, Option<usize>)) {
        self.0.push((name, range));
    }
}
