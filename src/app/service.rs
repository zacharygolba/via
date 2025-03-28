use hyper::body::Incoming;
use hyper::service::Service;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::app::App;
use crate::body::{HttpBody, RequestBody, ResponseBody};
use crate::middleware::{FutureResponse, Next};
use crate::request::Request;

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    future: FutureResponse,
}

impl<T> AppService<T> {
    #[inline(always)]
    pub(crate) fn new(app: Arc<App<T>>, max_body_size: usize) -> Self {
        Self { app, max_body_size }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = Infallible;
    type Future = ServeRequest;
    type Response = http::Response<HttpBody<ResponseBody>>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut request = Request::new(
            Arc::clone(&self.app.state),
            request.map(|body| RequestBody::new(self.max_body_size, body).into()),
        );

        let mut next = Next::new();

        for matching in self.app.router.visit(request.uri().path()) {
            for cond in matching.iter() {
                let node = *cond.as_either();

                if let Some(name) = node.param() {
                    request.params_mut().push(name.clone(), matching.range());
                }

                node.iter()
                    .filter_map(|route| cond.as_match(route))
                    .for_each(|middleware| {
                        next.stack_mut().push_back(Arc::clone(middleware));
                    });
            }
        }

        ServeRequest {
            future: next.call(request),
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
