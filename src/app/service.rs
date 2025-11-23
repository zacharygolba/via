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

use crate::app::Via;
use crate::middleware::BoxFuture;
use crate::next::Next;
use crate::request::{Envelope, Request};
use crate::response::{Response, ResponseBody};

pub struct ServeRequest(BoxFuture);

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

impl<App> Service<http::Request<Incoming>> for AppService<App>
where
    App: Send + Sync,
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

            // Limit the length of the request body to max_request_size.
            let body = Limited::new(body, self.max_request_size);

            Request::new(self.app.app.clone(), parts, body)
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
