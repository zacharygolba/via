use hyper::{body::Incoming, service::Service};
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{
    event::{Event, EventListener},
    middleware::BoxFuture,
    response::{Body, Response},
    router::Router,
    App, Request, Result,
};

pub struct AppService<State> {
    pub event_listener: EventListener,
    pub router: Arc<Router<State>>,
    pub state: Arc<State>,
}

pub struct FutureResponse {
    event_listener: EventListener,
    response_future: BoxFuture<Result<Response>>,
}

impl<State> AppService<State> {
    pub fn new<T>(app: App<State>, event_listener: T) -> Self
    where
        T: Fn(Event) + Send + Sync + 'static,
    {
        Self {
            event_listener: EventListener::new(event_listener),
            router: Arc::new(app.router),
            state: Arc::new(app.state),
        }
    }
}

impl<State> Clone for AppService<State> {
    fn clone(&self) -> Self {
        Self {
            event_listener: self.event_listener.clone(),
            router: Arc::clone(&self.router),
            state: Arc::clone(&self.state),
        }
    }
}

impl<State> Service<http::Request<Incoming>> for AppService<State>
where
    State: Send + Sync + 'static,
{
    type Error = Infallible;
    type Future = FutureResponse;
    type Response = http::Response<Body>;

    fn call(&self, request: http::Request<Incoming>) -> Self::Future {
        // Wrap the incoming request with `via::Request`.
        let mut request = Request::new(
            request,
            Arc::clone(&self.state),
            self.event_listener.clone(),
        );
        // Route the request to the appropriate middleware stack.
        let next = self.router.visit(&mut request);

        Self::Future {
            // Clone the event listener to pass to the future. This is used to
            // notify the event listener of any recoverable errors that occur
            // while processing the request.
            event_listener: self.event_listener.clone(),
            // Unwind the middleware stack and pass the returned future to our
            // custom future type. We'll use this future to poll the middleware
            // stack and generate a response for the request.
            response_future: next.call(request),
        }
    }
}

impl FutureResponse {
    fn project(
        self: Pin<&mut Self>,
    ) -> (
        Pin<&EventListener>,
        Pin<&mut dyn Future<Output = Result<Response>>>,
    ) {
        // Safety:
        // This block is necessary because we need to project the fields of
        // `FutureResponse` through the pinned reference.
        unsafe {
            // Get a mutable reference to the struct from the pinned box.
            let this = self.get_unchecked_mut();
            // `EventListener` is not pinned and does not contain any `Unpin`
            // fields. Therefore, it is safe to pin it.
            let event_listener = &this.event_listener;
            // Get a mutable reference to the inner `Future` from the pinned
            // box. This is safe because the box is pinned, so the `Future`
            // cannot move.
            let response_future = this.response_future.as_mut().get_unchecked_mut();

            // Return the pinned references to `EventListener` and `Future`.
            (
                Pin::new_unchecked(event_listener),
                Pin::new_unchecked(response_future),
            )
        }
    }
}

impl Future for FutureResponse {
    type Output = Result<http::Response<Body>, Infallible>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let (event_listener, response_future) = self.project();

        match response_future.poll(context) {
            Poll::Pending => {
                // The response is not ready yet. Return pending.
                Poll::Pending
            }
            Poll::Ready(Err(error)) => {
                // An error occurred in the middleware stack. We'll convert the
                // error into a response with the configured error format. If we
                // are unable to convert the error into a response, we'll fallback
                // to a plain text response and propagate the error to the event
                // listener so it can be handled at the application level.
                let response = error.into_infallible_response(|error| {
                    // Notify the event listener that an uncaught error occurred.
                    event_listener.call(Event::UncaughtError(error));
                });

                // Unwrap the inner response from `via::Response` and return ready.
                Poll::Ready(Ok(response.into_inner()))
            }
            Poll::Ready(Ok(response)) => {
                // The response was successfully generated. Unwrap the inner
                // response from `via::Response` and return ready.
                Poll::Ready(Ok(response.into_inner()))
            }
        }
    }
}
