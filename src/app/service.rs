use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::Pattern;

use crate::BoxBody;
use crate::app::App;
use crate::middleware::{BoxFuture, Next};
use crate::request::param::PathParams;
use crate::request::{Head, Request};
use crate::response::ResponseBody;

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    result: Result<BoxFuture, Infallible>,
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
    type Response = http::Response<ResponseBody>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut params = PathParams::new(Vec::with_capacity(8));
        let mut next = Next::new(VecDeque::new());

        let path = request.uri().path();

        for binding in self.app.router.visit(path) {
            for node in binding.results() {
                let match_as_final = match (node.pattern(), binding.range()) {
                    (Pattern::Dynamic(name), Some(range)) => {
                        params.push(name.clone(), *range);
                        binding.is_final()
                    }
                    (Pattern::Wildcard(name), None) => {
                        params.push(name.clone(), [path.len(); 2]);
                        true
                    }
                    (Pattern::Wildcard(name), Some([start, _])) => {
                        params.push(name.clone(), [*start, path.len()]);
                        true
                    }
                    _ => binding.is_final(),
                };

                if match_as_final {
                    // Include the entire route stack of the matched node.
                    next.extend(node.as_final().cloned());
                } else {
                    // Only include the middleware of the matched node.
                    next.extend(node.as_partial().cloned());
                }
            }
        }

        ServeRequest {
            result: Ok(next.call({
                let (parts, body) = request.into_parts();

                Request::new(
                    Arc::clone(&self.app.state),
                    Head::new(parts, params),
                    BoxBody::new(Limited::new(body, self.max_body_size)),
                )
            })),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match &mut self.result {
            Ok(future) => match future.as_mut().poll(context) {
                Poll::Ready(Ok(response)) => Poll::Ready(Ok(response.into())),
                Poll::Ready(Err(error)) => Poll::Ready(Ok(error.into())),
                Poll::Pending => Poll::Pending,
            },
            Err(error) => Poll::Ready(Err(*error)),
        }
    }
}
