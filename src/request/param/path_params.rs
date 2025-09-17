use std::sync::Arc;
use via_router::Param;

use super::path_param::PathParam;

#[derive(Debug)]
pub(crate) struct PathParams {
    params: Vec<(Arc<str>, Param)>,
}

#[cfg(feature = "ws")]
#[derive(Debug)]
pub struct OwnedPathParams {
    offsets: PathParams,
    uri: http::Uri,
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

#[cfg(feature = "ws")]
impl OwnedPathParams {
    pub(crate) fn new(uri: http::Uri, offsets: PathParams) -> Self {
        Self { offsets, uri }
    }

    #[inline]
    pub fn get<'b>(&self, name: &'b str) -> PathParam<'_, 'b> {
        self.offsets.get(self.uri().path(), name)
    }

    #[inline]
    pub fn uri(&self) -> &http::Uri {
        &self.uri
    }
}
