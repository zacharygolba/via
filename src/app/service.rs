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
use crate::request::{PathParams, Request, RequestBody, RequestHead};
use crate::response::{Response, ResponseBody};

pub struct ServeRequest(BoxFuture);

pub struct AppService<State> {
    app: Arc<App<State>>,
    max_request_size: usize,
}

impl<State> AppService<State> {
    #[inline]
    pub(crate) fn new(app: Arc<App<State>>, max_request_size: usize) -> Self {
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

impl<State> Service<http::Request<Incoming>> for AppService<State>
where
    State: Send + Sync,
{
    type Error = Infallible;
    type Future = ServeRequest;
    type Response = http::Response<ResponseBody>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        // The middleware stack.
        let mut deque = VecDeque::new();

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

        // Get a reference to the component parts of the request as well as a
        // mutable reference to the path parameters.
        let RequestHead {
            ref mut params,
            ref parts,
            ..
        } = *request.head_mut();

        // Populate the middleware stack with the resolved routes.
        for (route, param) in self.app.router.traverse(parts.uri.path()) {
            // Extend the deque with the matching route's middleware.
            deque.extend(route.cloned());

            if let Some((name, range)) = param {
                // Include the route's dynamic parameter in params.
                params.push(Arc::clone(name), range);
            }
        }

        // Call the middleware stack to get a response.
        ServeRequest(Next::new(deque).call(request))
    }
}

impl Future for ServeRequest {
    type Output = Result<http::Response<ResponseBody>, Infallible>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let Self(future) = self.get_mut();

        future
            .as_mut()
            .poll(context)
            .map(|result| Ok(result.unwrap_or_else(Response::from).into()))
    }
}
