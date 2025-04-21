use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use via_router::binding::MatchCond;
use via_router::MatchKind;

use crate::middleware::{BoxFuture, Next};
use crate::request::{Head, Params, Request};
use crate::response::ResponseBody;
use crate::BoxBody;

use super::router::Router;

pub struct ServeRequest(Result<BoxFuture, via_router::Error>);

pub struct AppService<T> {
    service: Arc<AppServiceBase<T>>,
}

struct AppServiceBase<T> {
    state: Arc<T>,
    router: Router<T>,
    max_body_size: usize,
    max_connections: usize,
    shutdown_timeout: Duration,
}

impl<T> AppService<T> {
    pub(crate) fn new(
        state: T,
        router: Router<T>,
        max_body_size: usize,
        max_connections: usize,
        shutdown_timeout: Duration,
    ) -> Self {
        Self {
            service: Arc::new(AppServiceBase {
                state: Arc::new(state),
                router,
                max_body_size,
                max_connections,
                shutdown_timeout,
            }),
        }
    }

    pub(crate) fn max_connections(&self) -> usize {
        self.service.max_connections
    }

    pub(crate) fn shutdown_timeout(&self) -> Duration {
        self.service.shutdown_timeout
    }
}

impl<T> Clone for AppService<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            service: Arc::clone(&self.service),
        }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = via_router::Error;
    type Future = ServeRequest;
    type Response = http::Response<ResponseBody>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut params = Params::new();
        let mut next = Next::new(VecDeque::with_capacity(7));
        let path = request.uri().path();

        let results = match self.service.router.visit(&path) {
            Err(error) => return ServeRequest(Err(error)),
            Ok(visted) => visted,
        };

        for binding in &results {
            for kind in binding.nodes() {
                let param = match kind {
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
                };

                if let Some((name, range)) = param {
                    params.insert(name, range);
                }
            }
        }

        ServeRequest(Ok(next.call({
            let (parts, body) = request.into_parts();

            Request::new(
                Arc::clone(&self.service.state),
                Head::new(parts, params),
                BoxBody::new(Limited::new(body, self.service.max_body_size)),
            )
        })))
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, via_router::Error>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match &mut self.0 {
            Ok(future) => match future.as_mut().poll(context) {
                Poll::Ready(Ok(response)) => Poll::Ready(Ok(response.into())),
                Poll::Ready(Err(error)) => Poll::Ready(Ok(error.into())),
                Poll::Pending => Poll::Pending,
            },
            Err(error) => Poll::Ready(Err(*error)),
        }
    }
}
