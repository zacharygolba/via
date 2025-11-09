use http::request::Parts;
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
use crate::request::{Head, Request};
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

            // The request type owns an Arc to the application state.
            let state = self.app.state.clone();

            // Limit the length of the request body to max_request_size.
            let body = Limited::new(body, self.max_request_size);

            Request::new(state, parts, body)
        };

        // Borrow the request params mutably and borrow the uri.
        let Head {
            ref mut params,
            parts: Parts { ref uri, .. },
            ..
        } = *request.head_mut();

        // Populate the middleware stack with the resolved routes.
        for (route, param) in self.app.router.traverse(uri.path()) {
            // Extend the deque with the matching route's middleware.
            deque.extend(route.cloned());

            if let Some((name, range)) = param {
                // Include the route's dynamic parameter in params.
                params.push((Arc::clone(name), range));
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
        future.as_mut().poll(context).map(|result| {
            Ok(match result {
                Ok(response) => response.into(),
                Err(error) => Response::from(error).into(),
            })
        })
    }
}
