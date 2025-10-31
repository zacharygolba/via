use std::sync::Arc;
use via_router::Param;

use super::path_param::PathParam;

#[derive(Debug)]
pub(crate) struct PathParams {
    params: Vec<(Arc<str>, Param)>,
}

fn range_for(predicate: &str, slice: &[(Arc<str>, Param)]) -> Option<(usize, Option<usize>)> {
    slice.iter().find_map(|(name, range)| {
        if predicate == name.as_ref() {
            Some(*range)
        } else {
            None
        }
    })
}

impl PathParams {
    #[inline]
    pub(crate) fn new(params: Vec<(Arc<str>, Param)>) -> Self {
        Self { params }
    }

    #[inline]
    pub(crate) fn get<'a, 'b>(&self, source: &'a str, name: &'b str) -> PathParam<'a, 'b> {
        PathParam::new(name, source, range_for(name, &self.params))
    }

    #[inline]
    pub(crate) fn push(&mut self, name: Arc<str>, range: Param) {
        self.params.push((name, range));
    }
}
