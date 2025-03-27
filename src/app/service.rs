use hyper::body::Incoming;
use hyper::service::Service;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use via_router::Router;

use crate::app::App;
use crate::body::{HttpBody, RequestBody, ResponseBody};
use crate::error::DynError;
use crate::middleware::FutureResponse;
use crate::request::Request;
use crate::router::MatchWhen;
use crate::{Middleware, Next};

pub struct AppService<T> {
    state: Arc<T>,
    router: Router<Box<dyn Middleware<T>>>,
    max_body_size: usize,
}

pub struct ServeRequest {
    future: FutureResponse,
}

impl<T> AppService<T> {
    #[inline(always)]
    pub(crate) fn new(app: App<T>, max_body_size: usize) -> Self {
        Self {
            state: app.state,
            router: app.router.build(),
            max_body_size,
        }
    }
}

impl<T: Send + Sync> Service<http::Request<Incoming>> for AppService<T> {
    type Error = DynError;
    type Future = ServeRequest;
    type Response = http::Response<HttpBody<ResponseBody>>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        let mut request = Request::new(
            Arc::clone(&self.state),
            request.map(|body| RequestBody::new(self.max_body_size, body).into()),
        );
        let mut next = Next::new();
        let stack = next.stack_mut();

        for binding in self.router.visit(request.uri().path()) {
            if let Some((name, range)) = binding.param() {
                request.params_mut().push((name.clone(), range));
            }
        }

        ServeRequest {
            result: Ok(next.call(request)),
        }
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<HttpBody<ResponseBody>>, Error>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let pending = match &mut self.result {
            Err(error) => return Poll::Ready(Err(error.clone())),
            Ok(future) => future,
        };

        let response = match pending.as_mut().poll(context) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => e.into(),
            Poll::Ready(Ok(response)) => response,
        };

        Poll::Ready(Ok(response.into_inner()))
    }
}
