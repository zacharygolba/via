use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
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

pub struct ServeRequest(BoxFuture);

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_body_size: usize,
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
        // Allocate early for path parameters to confirm that we are able to
        // perform an allocation before serving the request.
        //
        // It's safer to fail here than later on when application specific
        // business logic takes over.
        let mut params = PathParams::new(Vec::with_capacity(8));

        // Dynamically allocate to store the middleware stack for the request.
        let mut next = Next::new(VecDeque::new());

        // The request type owns an Arc to the global application state. This
        // requires an atomic op. Performing it early is likely beneficial to
        // synchronize "the state of the world" before routing the request.
        // Every other atomic op performed is conditional.
        let state = Arc::clone(&self.app.state);

        // 1 atomic op per matched middleware fn and an additional op if the
        // path segment matched a dynamic segment.
        //
        // Allocations only occur if a path segment has 2 :dynamic or *wildcard
        // patterns and a static a static pattern that matches the path
        // segment.
        for (stack, param) in self.app.router.visit(request.uri().path()) {
            next.extend(stack.map(Arc::clone));
            if let Some((name, range)) = param {
                params.push(Arc::clone(name), range);
            }
        }

        let (parts, body) = request.into_parts();
        let request = Request::new(
            state,
            Head::new(parts, params),
            BoxBody::new(Limited::new(body, self.max_body_size)),
        );

        ServeRequest(next.call(request))
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        match self.0.as_mut().poll(context) {
            Poll::Ready(Ok(response)) => Poll::Ready(Ok(response.into())),
            Poll::Ready(Err(error)) => Poll::Ready(Ok(error.into())),
            Poll::Pending => Poll::Pending,
        }
    }
}
