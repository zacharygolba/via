use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

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
        let mut params = PathParams::new(Vec::with_capacity(4));
        let mut next = Next::new(VecDeque::with_capacity(6));
        let request = request.map(|body| HttpBody::request(self.max_body_size, body));
        let state = Arc::clone(&self.app.state);

        for binding in self.app.visit(request.uri().path()) {
            for matched in binding.iter() {
                if let Some(name) = matched.param() {
                    params.push(name.clone(), binding.range());
                }

                for middleware in matched.iter() {
                    next.push(Arc::clone(middleware));
                }
            }
        }

        ServeRequest {
            future: next.call(Request::new(state, params, request)),
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
