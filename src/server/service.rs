use http::StatusCode;
use hyper::body::Incoming;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use crate::body::Pollable;
use crate::middleware::BoxFuture;
use crate::router::Router;
use crate::{Error, Request, Response};

/// The request type used by our hyper service. This is the type that we will
/// wrap in a `via::Request` and pass to the middleware stack.
type HttpRequest = http::Request<Incoming>;

/// The response type used by our hyper service. This is the type that we will
/// unwrap from the `via::Response` returned from the middleware stack.
type HttpResponse = http::Response<Pollable>;

pub struct FutureResponse {
    future: BoxFuture<Result<Response, Error>>,
    started: Instant,
    timeout: Duration,
}

pub struct Service<State> {
    router: Arc<Router<State>>,
    state: Arc<State>,
    timeout: Duration,
}

/// Returns a response with a 504 Gateway Timeout status code. This is used if
/// `ResponseFuture` is polled for longer than the configured timeout.
fn respond_with_timeout() -> HttpResponse {
    let mut message = String::with_capacity(65);
    let status = StatusCode::GATEWAY_TIMEOUT;

    message.push_str("The server is taking too long to respond. ");
    message.push_str("Please try again later.");

    Error::with_status(message, status)
        .into_response()
        .into_inner()
}

impl Future for FutureResponse {
    type Output = Result<HttpResponse, Infallible>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        if this.started.elapsed() >= this.timeout {
            return Poll::Ready(Ok(respond_with_timeout()));
        }

        this.future
            .as_mut()
            .poll(context)
            .map(|result| match result {
                // The response was generated successfully.
                Ok(response) => Ok(response.into_inner()),
                // An occurred while generating the response.
                Err(error) => Ok(error.into_response().into_inner()),
            })
    }
}

impl<State> Service<State> {
    pub fn new(router: Arc<Router<State>>, state: Arc<State>, timeout: Duration) -> Self {
        Self {
            router,
            state,
            timeout,
        }
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
        // `Result<Self::Response, Self::Error>`. If the Future is polled for
        // longer than the configured timeout, we'll respond with a 504 Gateway
        // Timeout instead.
        Self::Future {
            future: next.call(request),
            started: Instant::now(),
            timeout: self.timeout,
        }
    }
}
