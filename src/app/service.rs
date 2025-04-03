use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::{MatchCond, MatchKind};

use crate::app::App;
use crate::body::{HttpBody, RequestBody, ResponseBody};
use crate::middleware::{FutureResponse, Next};
use crate::request::param::PathParams;
use crate::request::Request;

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    response: FutureResponse,
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
        let mut next = Next::new(VecDeque::with_capacity(6));
        let path = request.uri().path();

        for binding in self.app.router.visit(path) {
            for match_kind in binding.nodes() {
                match match_kind {
                    MatchKind::Edge(MatchCond::Partial(node)) => {
                        next.extend(node.route().filter_map(MatchCond::as_partial).cloned());
                        if let Some((name, range)) = node.param().zip(binding.range()) {
                            params.push(name.clone(), *range);
                        }
                    }
                    MatchKind::Edge(MatchCond::Exact(node)) => {
                        next.extend(node.route().map(MatchCond::as_either).cloned());
                        if let Some((name, range)) = node.param().zip(binding.range()) {
                            params.push(name.clone(), *range);
                        }
                    }
                    MatchKind::Wildcard(node) => {
                        next.extend(node.route().map(MatchCond::as_either).cloned());
                        if let Some((name, [start, _])) = node.param().zip(binding.range()) {
                            params.push(name.clone(), [*start, path.len()]);
                        }
                    }
                }
            }
        }

        ServeRequest {
            response: next.call({
                let (parts, body) = request.into_parts();
                let body = HttpBody::Inline(RequestBody::new(self.max_body_size, body));
                Request::new(Arc::clone(&self.app.state), params, parts, body)
            }),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<HttpBody<ResponseBody>>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.response
            .as_mut()
            .poll(context)
            .map(|result| Ok(result.unwrap_or_else(|e| e.into()).into_inner()))
    }
}
