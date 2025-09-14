use http::uri::PathAndQuery;
use std::sync::Arc;
use via_router::Param;

use super::path_param::PathParam;

#[derive(Debug)]
pub(crate) struct ParamOffsets(Vec<(Arc<str>, Param)>);

#[derive(Debug)]
pub struct Params {
    path_and_query: Option<PathAndQuery>,
    offsets: ParamOffsets,
}

impl ParamOffsets {
    #[inline]
    pub(crate) fn new(data: Vec<(Arc<str>, Param)>) -> Self {
        Self(data)
    }

    #[inline]
    pub(crate) fn get<'a, 'b>(&self, source: &'a str, name: &'b str) -> PathParam<'a, 'b> {
        PathParam::new(
            name,
            source,
            self.0
                .iter()
                .find_map(|(k, v)| if &**k == name { Some(*v) } else { None }),
        )
    }

    #[inline]
    pub(crate) fn push(&mut self, name: Arc<str>, range: Param) {
        self.0.push((name, range));
    }
}

impl Params {
    #[inline]
    pub fn get<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.offsets.get(self.path(), name)
    }

    #[inline]
    pub fn path(&self) -> &str {
        self.path_and_query.as_ref().map_or("/", PathAndQuery::path)
    }
}

impl Params {
    pub(crate) fn new(path_and_query: Option<PathAndQuery>, offsets: ParamOffsets) -> Self {
        Self {
            path_and_query,
            offsets,
        }
    }
}
