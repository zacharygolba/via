use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::{MatchCond, MatchKind, Param};

use crate::app::App;
use crate::body::{HttpBody, ResponseBody};
use crate::middleware::{FutureResponse, Next};
use crate::request::{PathParams, Request};

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    future: FutureResponse,
}

/// Lazily increments the ref count for the param name if Some, a copy of the
/// provided range will be zipped with the param name.
///
fn lazy_clone_param(
    param: Option<&Param>,
    range: Option<&[usize; 2]>,
) -> Option<(Param, [usize; 2])> {
    match (param, range) {
        (Some(name), Some(&at)) => Some((name.clone(), at)),
        (None, None) | (None, Some(_)) | (Some(_), None) => None,
    }
}

impl<T> AppService<T> {
    #[inline]
    pub(crate) fn new(app: Arc<App<T>>, max_body_size: usize) -> Self {
        Self { app, max_body_size }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = Infallible;
    type Future = ServeRequest;
    type Response = http::Response<HttpBody<ResponseBody>>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut params = PathParams::new(Vec::with_capacity(6));
        let mut next = Next::new(VecDeque::new());

        let max_body_size = self.max_body_size;
        let state = Arc::clone(&self.app.state);
        let path = request.uri().path();

        for binding in self.app.router.visit(path) {
            for match_kind in binding.nodes() {
                match match_kind {
                    MatchKind::Edge(MatchCond::Partial(partial)) => {
                        next.extend(partial.route().filter_map(MatchCond::as_partial).cloned());
                        params.extend(lazy_clone_param(partial.param(), binding.range()));
                    }
                    MatchKind::Edge(MatchCond::Exact(exact)) => {
                        next.extend(exact.route().map(MatchCond::as_either).cloned());
                        params.extend(lazy_clone_param(exact.param(), binding.range()));
                    }
                    MatchKind::Wildcard(wildcard) => {
                        next.extend(wildcard.route().map(MatchCond::as_either).cloned());
                        params.extend(lazy_clone_param(wildcard.param(), binding.range()).map(
                            |(name, mut range)| {
                                std::mem::swap(&mut range[1], &mut path.len());
                                (name, range)
                            },
                        ));
                    }
                }
            }
        }

        ServeRequest {
            future: next.call(Request::new(
                state,
                params,
                request.map(|body| HttpBody::request(max_body_size, body)),
            )),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<HttpBody<ResponseBody>>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.future
            .as_mut()
            .poll(context)
            .map(|result| Ok(result.unwrap_or_else(|e| e.into()).into_inner()))
    }
}
