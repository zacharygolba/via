use hyper::body::Incoming;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::body::{Boxed, Buffered, Either};
use crate::middleware::BoxFuture;
use crate::router::Router;
use crate::{Error, Request, Response};

/// The request type used by our hyper service. This is the type that we will
/// wrap in a `via::Request` and pass to the middleware stack.
type HttpRequest = http::Request<Incoming>;

/// The response type used by our hyper service. This is the type that we will
/// unwrap from the `via::Response` returned from the middleware stack.
type HttpResponse = http::Response<Either<Buffered, Boxed>>;

pub struct FutureResponse {
    future: BoxFuture<Result<Response, Error>>,
}

pub struct Service<State> {
    router: Arc<Router<State>>,
    state: Arc<State>,
}

impl Future for FutureResponse {
    type Output = Result<HttpResponse, Infallible>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        this.future
            .as_mut()
            .poll(context)
            .map(|result| match result {
                // The response was generated successfully.
                Ok(response) => Ok(response.into()),
                // An occurred while generating the response.
                Err(error) => Ok(error.into_response().into()),
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

    fn call(&self, request: HttpRequest) -> Self::Future {
        // Get a Vec of routes that match the uri path.
        let matched_routes = self.router.lookup(request.uri().path());

        // Build the middleware stack for the request.
        let (params, next) = self.router.resolve(&matched_routes);

        // Wrap the incoming request in with `via::Request`.
        let request = Request::new(request, params, Arc::clone(&self.state));

        // Call the middleware stack and return a Future that resolves to
        // `Result<Self::Response, Self::Error>`.
        Self::Future {
            future: next.call(request),
        }
    }
}
