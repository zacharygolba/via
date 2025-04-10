use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::binding::MatchCond;
use via_router::MatchKind;

use crate::app::App;
use crate::middleware::{BoxFuture, Next};
use crate::request::param::PathParams;
use crate::request::{Head, Request};
use crate::response::ResponseBody;
use crate::BoxBody;

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    response: Result<BoxFuture, via_router::Error>,
}

impl<T> AppService<T> {
    #[inline]
    pub(crate) fn new(app: Arc<App<T>>, max_body_size: usize) -> Self {
        Self { app, max_body_size }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = via_router::Error;
    type Future = ServeRequest;
    type Response = http::Response<ResponseBody>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut params = PathParams::new(Vec::with_capacity(7));
        let mut next = Next::new(VecDeque::with_capacity(7));

        let path = request.uri().path();
        let hits = match self.app.router.visit(path) {
            Ok(visted) => visted,
            Err(error) => {
                return ServeRequest {
                    response: Err(error),
                }
            }
        };

        for binding in &hits {
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
            response: Ok(next.call({
                let (parts, body) = request.into_parts();

                Request::new(
                    Arc::clone(&self.app.state),
                    Box::new(Head::new(parts, params)),
                    BoxBody::new(Limited::new(body, self.max_body_size)),
                )
            })),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, via_router::Error>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match &mut self.response {
            Ok(future) => match future.as_mut().poll(context) {
                Poll::Ready(Ok(response)) => Poll::Ready(Ok(response.into())),
                Poll::Ready(Err(error)) => Poll::Ready(Ok(error.into())),
                Poll::Pending => Poll::Pending,
            },
            Err(error) => Poll::Ready(Err(*error)),
        }
    }
}
