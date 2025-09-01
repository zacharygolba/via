use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

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
                if let Some(param) = node.param().cloned()
                    && let Some(range) = binding.range()
                {
                    if node.is_wildcard() {
                        params.push(param, [range[0], path.len()]);
                    } else {
                        params.push(param, *range);
                    }
                }

                if node.is_wildcard() || binding.is_final() {
                    next.extend(node.exact().cloned());
                } else {
                    next.extend(node.partial().cloned());
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
