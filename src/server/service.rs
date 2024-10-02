use hyper::body::Incoming;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::body::{AnyBody, ByteBuffer};
use crate::middleware::BoxFuture;
use crate::request::{PathParams, Request, RequestBody};
use crate::router::{self, Router};
use crate::{Error, Next, Response};

/// The request type used by our hyper service. This is the type that we will
/// wrap in a `via::Request` and pass to the middleware stack.
type HttpRequest = http::Request<Incoming>;

/// The response type used by our hyper service. This is the type that we will
/// unwrap from the `via::Response` returned from the middleware stack.
type HttpResponse = http::Response<AnyBody<ByteBuffer>>;

pub struct FutureResponse {
    future: BoxFuture<Result<Response, Error>>,
}

pub struct Service<State> {
    router: Arc<Router<State>>,
    state: Arc<State>,
}

impl Future for FutureResponse {
    type Output = Result<HttpResponse, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        self.future
            .as_mut()
            .poll(context)
            .map(|result| match result {
                // The response was generated successfully.
                Ok(response) => Ok(response.into_outgoing_response()),
                // An occurred while generating the response.
                Err(error) => Ok(error.into_response().into_outgoing_response()),
            })
    }
}

impl<State> Service<State> {
    pub fn new(router: Arc<Router<State>>, state: Arc<State>) -> Self {
        Self { router, state }
    }
}

impl<State> hyper::service::Service<HttpRequest> for Service<State>
where
    State: Send + Sync + 'static,
{
    type Error = Infallible;
    type Future = FutureResponse;
    type Response = HttpResponse;

    fn call(&self, incoming: HttpRequest) -> Self::Future {
        // Wrap the incoming request in with `via::Request`.
        let mut request = {
            // Destructure the incoming request into its component parts.
            let (parts, body) = incoming.into_parts();

            // Allocate the metadata associated with the request on the heap.
            // This keeps the size of the request type small enough to pass
            // around by value.
            let parts = Box::new(parts);

            // Wrap the request body with `RequestBody`.
            let body = RequestBody::new(AnyBody::Inline(body));

            // Clone the shared application state so request can own a reference
            // to it. This is a cheaper operation than going from Weak to Arc for
            // any application that interacts with a connection pool or an
            // external service.
            let state = Arc::clone(&self.state);

            // Allocate a vec to store the path parameters associated with the
            // request.
            let params = PathParams::new(Vec::new());

            Request::new(parts, body, state, params)
        };

        // Build the middleware stack for the request.
        let next = {
            // Get an iterator of matched nodes for the uri path.
            let mut visited = {
                let path = request.uri().path();
                self.router.lookup(path).rev()
            };

            // Allocate a vec to store the middleware associated with the
            // request.
            let mut stack = Vec::new();

            // Get a mutable reference to the request's path parameters.
            let params = request.params_mut();

            // Populate the path params and build middleware stack.
            router::resolve(&mut stack, params, &mut visited);

            Next::new(stack)
        };

        // Call the middleware stack and return a Future that resolves to
        // `Result<Self::Response, Self::Error>`.
        Self::Future {
            future: next.call(request),
        }
    }
}
