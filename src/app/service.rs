use http::request::Parts;
use hyper::body::Incoming;
use hyper::service::Service;
use std::collections::VecDeque;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::middleware::BoxFuture;
use crate::request::{Envelope, Request};
use crate::response::{Response, ResponseBody};
use crate::{Next, Via};

pub struct FutureResponse(BoxFuture);

pub struct AppService<App> {
    app: Arc<Via<App>>,
    max_request_size: usize,
}

impl<App> AppService<App> {
    #[inline]
    pub(crate) fn new(app: Arc<Via<App>>, max_request_size: usize) -> Self {
        Self {
            app,
            max_request_size,
        }
    }
}

impl<App> Clone for AppService<App> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            app: Arc::clone(&self.app),
            max_request_size: self.max_request_size,
        }
    }
}

impl<App> Service<http::Request<Incoming>> for AppService<App> {
    type Error = Infallible;
    type Future = FutureResponse;
    type Response = http::Response<ResponseBody>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        // The middleware stack.
        let mut deque = VecDeque::new();

        // Wrap the raw HTTP request in our custom Request struct.
        let mut request = {
            // Preallocate 240 bytes for path parameter ranges.
            let params = Vec::with_capacity(6);

            // Ownership of app is shared with Request.
            let app = self.app.app.clone();

            Request::new(app, self.max_request_size, params, request)
        };

        // Borrow the request params mutably and borrow the uri.
        let Envelope {
            ref mut params,
            parts: Parts { ref uri, .. },
            ..
        } = *request.envelope_mut();

        // Populate the middleware stack with the resolved routes.
        for (route, param) in self.app.router.traverse(uri.path()) {
            // Extend the deque with the route's middleware stack.
            deque.extend(route.cloned());

            if let Some((name, range)) = param {
                // Include the route's dynamic parameter in params.
                params.push((Arc::clone(name), range));
            }
        }

        // Call the middleware stack to get a response.
        FutureResponse(Next::new(deque).call(request))
    }
}

impl Future for FutureResponse {
    type Output = Result<http::Response<ResponseBody>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.0
            .as_mut()
            .poll(context)
            .map(|result| Ok(result.unwrap_or_else(Response::from).into()))
    }
}
