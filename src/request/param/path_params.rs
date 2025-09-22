#[cfg(feature = "ws")]
use http::uri::PathAndQuery;
use std::sync::Arc;
use via_router::Param;

use super::path_param::PathParam;

#[derive(Debug)]
pub(crate) struct PathParams {
    params: Vec<(Arc<str>, Param)>,
}

#[cfg(feature = "ws")]
#[derive(Clone, Debug)]
pub struct OwnedPathParams {
    path_and_query: Option<PathAndQuery>,
    offsets: Arc<[(Arc<str>, Param)]>,
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

#[cfg(feature = "ws")]
impl OwnedPathParams {
    pub(crate) fn new(path_and_query: Option<PathAndQuery>, offsets: PathParams) -> Self {
        Self {
            path_and_query,
            offsets: offsets.params.into(),
        }
    }

    #[inline]
    pub fn get<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        PathParam::new(name, self.path(), range_for(name, &self.offsets))
    }

    #[inline]
    pub fn path(&self) -> &str {
        self.path_and_query.as_ref().map_or("/", PathAndQuery::path)
    }

    #[inline]
    pub fn query(&self) -> &str {
        self.path_and_query
            .as_ref()
            .and_then(PathAndQuery::query)
            .unwrap_or_default()
    }
}
