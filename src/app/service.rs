use http_body_util::Limited;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::app::App;
use crate::middleware::BoxFuture;
use crate::next::Next;
use crate::request::param::PathParams;
use crate::request::{Request, RequestBody, RequestHead};
use crate::response::ResponseBody;

pub struct ServeRequest(BoxFuture);

pub struct AppService<T> {
    app: Arc<App<T>>,
    max_request_size: usize,
}

impl<T> AppService<T> {
    #[inline]
    pub(crate) fn new(app: Arc<App<T>>, max_request_size: usize) -> Self {
        Self {
            app,
            max_request_size,
        }
    }
}

impl<State> Clone for AppService<State> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            app: Arc::clone(&self.app),
            max_request_size: self.max_request_size,
        }
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
                RequestHead::new(
                    parts,
                    // The request type owns an Arc to the application state.
                    // This is the only unconditional atomic op of the service.
                    Arc::clone(&self.app.state),
                    // Allocate early for path parameters to confirm that we are
                    // able to perform an allocation before serving the request.
                    //
                    // It's safer to fail here than later on when application
                    // specific business logic takes over.
                    PathParams::new(Vec::with_capacity(8)),
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
                RequestBody::new(Limited::new(body, self.max_request_size)),
            )
        };

        // Preallocate for the middleware stack.
        //
        // In the future, we can cache lazily resolved middleware stacks to
        // avoid this allocation and limit the atomic operations that occur
        // during route resolution to 1 per dynamic param name.
        let mut next = Next::new(VecDeque::with_capacity(8));

        // Get a mutable reference to the component parts of the request as
        // well as the vec that contains the path parameters.
        let RequestHead { parts, params, .. } = request.head_mut();

        // 1 atomic op per matched middleware fn and an additional op if the
        // path segment matched a dynamic segment.
        //
        // Allocations only occur if a path segment has 2 :dynamic or *wildcard
        // patterns and a static a static pattern that matches the path
        // segment.
        for (route, param) in self.app.router.traverse(parts.uri.path()) {
            // Add the matched route's middleware to the call stack.
            next.extend(route.map(Arc::clone));

            if let Some((name, range)) = param {
                // Include the resolved dynamic parameter in params.
                params.push(Arc::clone(name), range);
            }
        }

        // Call the middleware stack to get a response.
        ServeRequest(next.call(request))
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, Infallible>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let Self(future) = self.get_mut();

        if let Poll::Ready(result) = future.as_mut().poll(context) {
            let response = result.unwrap_or_else(|e| e.into());
            Poll::Ready(Ok(response.into()))
        } else {
            Poll::Pending
        }
    }
}
