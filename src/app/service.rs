use http_body_util::combinators::BoxBody;
use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::binding::MatchCond;
use via_router::MatchKind;

use crate::app::App;
use crate::middleware::{FutureResponse, Next};
use crate::request::param::PathParams;
use crate::request::{Head, Request};
use crate::response::ResponseBody;

pub struct AppService<T> {
    app: Arc<App<T>>,
    req_len_limit: usize,
}

pub struct ServeRequest {
    response: FutureResponse,
}

impl<T> AppService<T> {
    #[inline]
    pub(crate) fn new(app: Arc<App<T>>, req_len_limit: usize) -> Self {
        Self { app, req_len_limit }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = Infallible;
    type Future = ServeRequest;
    type Response = http::Response<ResponseBody>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut params = PathParams::new(Vec::with_capacity(7));
        let mut next = Next::new(VecDeque::with_capacity(7));

        let path = request.uri().path();

        for binding in self.app.router.visit(path) {
            for kind in binding.nodes() {
                params.extend(match kind {
                    MatchKind::Edge(MatchCond::Partial(node)) => {
                        next.extend(node.route().filter_map(MatchCond::as_partial).cloned());
                        node.param(|| binding.range())
                    }
                    MatchKind::Edge(MatchCond::Exact(node)) => {
                        next.extend(node.route().map(MatchCond::as_either).cloned());
                        node.param(|| binding.range())
                    }
                    MatchKind::Wildcard(node) => {
                        next.extend(node.route().map(MatchCond::as_either).cloned());
                        node.param(|| binding.range().map(|[start, _]| [start, path.len()]))
                    }
                });
            }
        }

        ServeRequest {
            response: next.call({
                let (parts, body) = request.into_parts();

                Request::new(
                    Arc::clone(&self.app.state),
                    Box::new(Head::new(parts, params)),
                    BoxBody::new(Limited::new(body, self.req_len_limit)),
                )
            }),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.response
            .as_mut()
            .poll(context)
            .map(|result| Ok(result.unwrap_or_else(|e| e.into()).inner))
    }
}
