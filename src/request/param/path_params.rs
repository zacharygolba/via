use http::uri::PathAndQuery;
use std::sync::Arc;
use via_router::Param;

use super::path_param::PathParam;

#[derive(Debug)]
pub(crate) struct PathParams {
    params: Vec<(Arc<str>, Param)>,
}

#[derive(Debug)]
pub struct OwnedPathParams {
    path_and_query: Option<PathAndQuery>,
    offsets: PathParams,
}

impl PathParams {
    #[inline]
    pub(crate) fn new(params: Vec<(Arc<str>, Param)>) -> Self {
        Self { params }
    }

    #[inline]
    pub(crate) fn get<'a, 'b>(&self, source: &'a str, name: &'b str) -> PathParam<'a, 'b> {
        let range = self
            .params
            .iter()
            .find_map(|(k, v)| if &**k == name { Some(*v) } else { None });

        PathParam::new(name, source, range)
    }

    #[inline]
    pub(crate) fn push(&mut self, name: Arc<str>, range: Param) {
        self.params.push((name, range));
    }
}

impl OwnedPathParams {
    #[inline]
    pub fn get<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.offsets.get(self.path(), name)
    }

    #[inline]
    pub fn path(&self) -> &str {
        self.path_and_query().map_or("/", PathAndQuery::path)
    }

    #[inline]
    pub fn query(&self) -> &str {
        self.path_and_query()
            .and_then(PathAndQuery::query)
            .unwrap_or_default()
    }
}

impl OwnedPathParams {
    pub(crate) fn new(path_and_query: Option<PathAndQuery>, offsets: PathParams) -> Self {
        Self {
            path_and_query,
            offsets,
        }
    }

    #[inline]
    fn path_and_query(&self) -> Option<&PathAndQuery> {
        self.path_and_query.as_ref()
    }
}
