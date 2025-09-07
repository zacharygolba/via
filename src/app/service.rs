use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::app::App;
use crate::middleware::{BoxFuture, Next};
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
        // Wrap the raw HTTP request in our custom Request struct.
        let mut request = {
            // Split the incoming request into it's component parts.
            let (parts, body) = request.into_parts();

            Request::new(
                Head::new(
                    parts,
                    // The request type owns an Arc to the application state.
                    // This is the only unconditional atomic op of the service.
                    Arc::clone(&self.app.state),
                    // Allocate early for path parameters to confirm that we are
                    // able to perform an allocation before serving the request.
                    //
                    // It's safer to fail here than later on when application
                    // specific business logic takes over.
                    HashMap::with_capacity(8),
                ),
                // Do not allocate for the request body until it's absolutely
                // necessary.
                //
                // They are buffered by default behind a channel. Therefore,
                // there is no risk of the request body overflowing the stack.
                //
                // This is also a small performance optimization that avoids an
                // additional allocation if you end up reading the entire body
                // into memory, a common case for backend JSON APIs.
                Limited::new(body, self.max_body_size),
            )
        };

        // Preallocate for the middleware stack.
        //
        // In the future, we can cache lazily resolved middleware stacks to
        // avoid this allocation and limit the atomic operations that occur
        // during route resolution to 1 per dynamic param name.
        let mut next = Next::new(VecDeque::with_capacity(8));

        // Get a mutable reference to the path params associated with the
        // request as well as a str to the uri path.
        let (params, path) = request.params_mut_with_path();

        // 1 atomic op per matched middleware fn and an additional op if the
        // path segment matched a dynamic segment.
        //
        // Allocations only occur if a path segment has 2 :dynamic or *wildcard
        // patterns and a static a static pattern that matches the path
        // segment.
        for (route, param) in self.app.router.traverse(path) {
            // Add the matched route's middleware to the call stack.
            next.extend(route.map(Arc::clone));

            if let Some((name, range)) = param {
                // Include the resolved dynamic parameter in params.
                params.insert(Arc::clone(name), range);
            }
        }

        // Call the middleware stack to get a response.
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
