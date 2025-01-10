use hyper::body::Incoming;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::body::{BufferBody, HttpBody};
use crate::error::Error;
use crate::middleware::BoxFuture;
use crate::request::{HyperBody, Request, RequestBody};
use crate::response::Response;
use crate::router::Router;

use super::shutdown::ShutdownTx;

pub struct FutureResponse {
    future: BoxFuture<Result<Response, Error>>,
}

pub struct Service<State> {
    max_request_size: usize,
    shutdown_tx: ShutdownTx,
    router: Arc<Router<State>>,
    state: Arc<State>,
}

impl Future for FutureResponse {
    type Output = Result<http::Response<HttpBody<BufferBody>>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.future
            .as_mut()
            .poll(context)
            .map(|result| match result {
                // The response was generated successfully.
                Ok(response) => Ok(response.into_inner()),
                // An occurred while generating the response.
                Err(error) => Ok(Response::from(error).into_inner()),
            })
    }
}

impl<State> Service<State> {
    pub fn new(
        max_request_size: usize,
        shutdown_tx: ShutdownTx,
        router: Arc<Router<State>>,
        state: Arc<State>,
    ) -> Self {
        Self {
            max_request_size,
            shutdown_tx,
            router,
            state,
        }
    }
}

impl<State> hyper::service::Service<http::Request<Incoming>> for Service<State>
where
    State: Send + Sync + 'static,
{
    type Error = Infallible;
    type Future = FutureResponse;
    type Response = http::Response<HttpBody<BufferBody>>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        // Wrap the incoming request in with `via::Request`.
        let mut request = {
            // Destructure the incoming request into its component parts.
            let (parts, incoming) = request.into_parts();

            // Allocate the metadata associated with the request on the heap.
            // This keeps the size of the request type small enough to pass
            // around by value.
            let parts = Box::new(parts);

            // Convert the incoming body type to a RequestBody.
            let body = RequestBody::new(
                self.max_request_size,
                HttpBody::Inline(HyperBody::new(incoming)),
            );

            // Clone the shared application state so request can own a reference
            // to it. This is a cheaper operation than going from Weak to Arc for
            // any application that interacts with a connection pool or an
            // external service.
            let state = Arc::clone(&self.state);

            Request::new(parts, body, state)
        };

        let future = match self
            .router
            .lookup(request.parts.uri.path(), &mut request.params)
        {
            // Call the middleware stack for the resolved routes.
            Ok(next) => next.call(request),

            // Alert the main thread that we encountered an unrecoverable error.
            Err(error) => {
                let _ = self.shutdown_tx.send(Some(true)).ok();
                let _ = error; // Placeholder for tracing...
                Box::pin(async { Ok(Response::internal_server_error()) })
            }
        };

        Self::Future { future }
    }
}
