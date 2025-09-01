use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
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
    result: Result<BoxFuture, via_router::Error>,
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
        let mut params = PathParams::new(Vec::with_capacity(8));
        let mut next = Next::new(VecDeque::new());

        let path = request.uri().path();

        for binding in self.app.router.visit(path) {
            for node in binding.results() {
                let path_pattern = node.pattern();
                let match_as_final = if let Pattern::Wildcard(name) = path_pattern {
                    if let Some([start, _]) = binding.range() {
                        params.push(name.clone(), [*start, path.len()]);
                    }
                    true
                } else if let Pattern::Dynamic(name) = path_pattern
                    && let Some(range) = binding.range().copied()
                {
                    params.push(name.clone(), range);
                    binding.is_final()
                } else {
                    binding.is_final()
                };

                if match_as_final {
                    next.extend(node.as_final().cloned());
                } else {
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
    type Output = Result<http::Response<ResponseBody>, via_router::Error>;

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
