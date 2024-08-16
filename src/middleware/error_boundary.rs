use std::sync::Arc;

use crate::event::{Event, EventListener};
use crate::middleware::BoxFuture;
use crate::{Error, Middleware, Next, Request, Response, Result};

/// Middleware that catches errors that occur in downstream middleware and
/// converts the error into a response. Upstream middleware added to an app
/// or endpoint before an `ErrorBoundary` will continue to execute as normal.
pub struct ErrorBoundary;

/// Works like `ErrorBoundary`, but allows you to map the error before it is
/// converted into a response. This can be useful for filtering out any
/// sensitive information from leaking into the response body.
pub struct MapErrorBoundary<F> {
    map: F,
}

/// Works like `ErrorBoundary` but allows you to inspect the error before it is
/// converted into a response. This is useful for logging the error message
/// or reporting the error to a monitoring service.
pub struct InspectErrorBoundary<F> {
    inspect: F,
}

/// Performs the conversion of an `Error` into a `Response`. If the conversion
/// fails, a fallback response is generated and `event_listener` is notified of
/// the error that prevented the response from being generated.
fn respond_with_error<State>(
    event_listener: &EventListener<State>,
    state: &Arc<State>,
    error: Error,
) -> Response {
    match error.try_into_response() {
        Ok(response) => response,
        Err((fallback_response, convert_error)) => {
            // Notify `event_listener` of the error that prevented the response
            // from being generated. This allows the error to be reported or
            // handled in the most suitable way for the application's needs.
            event_listener.call(Event::UncaughtError(&convert_error), state);

            // Return the fallback response that contains `error` as plain text.
            fallback_response
        }
    }
}

impl ErrorBoundary {
    pub fn inspect<State, F>(inspect: F) -> InspectErrorBoundary<F>
    where
        F: Fn(&Error, &Arc<State>) + Copy + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        InspectErrorBoundary { inspect }
    }

    pub fn map<State, F>(map: F) -> MapErrorBoundary<F>
    where
        F: Fn(Error, &Arc<State>) -> Error + Copy + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        MapErrorBoundary { map }
    }
}

impl<State> Middleware<State> for ErrorBoundary
where
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        Box::pin(async {
            // Copy `request.event_listener` so it can be used after ownership
            // of `request` is moved into `next.call()`. This is a cheap operation
            // since `EventListener` is a wrapper around a function pointer.
            let event_listener = *request.event_listener();
            // Clone `request.state` so it can be used after ownership of
            // `request` is moved into `next.call()`.
            let state = Arc::clone(request.state());

            next.call(request).await.or_else(|error| {
                // Convert the error to a response and return `Ok`.
                Ok(respond_with_error(&event_listener, &state, error))
            })
        })
    }
}

impl<State, F> Middleware<State> for MapErrorBoundary<F>
where
    F: Fn(Error, &Arc<State>) -> Error + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let map = self.map;

        Box::pin(async move {
            // Copy `request.event_listener` so it can be used after ownership
            // of `request` is moved into `next.call()`. This is a cheap operation
            // since `EventListener` is a wrapper around a function pointer.
            let event_listener = *request.event_listener();
            // Clone `request.state` so it can be used after ownership of
            // `request` is moved into `next.call()`.
            let state = Arc::clone(request.state());

            next.call(request).await.or_else(|error| {
                // Apply the `map` function to `error`. This allows the error
                // to be configured to use a different response format or to
                // filter out sensitive information from leaking into the
                // response body.
                let mapped = map(error, &state);

                // Convert the mapped error to a response and return `Ok`.
                Ok(respond_with_error(&event_listener, &state, mapped))
            })
        })
    }
}

impl<State, F> Middleware<State> for InspectErrorBoundary<F>
where
    F: Fn(&Error, &Arc<State>) + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let inspect = self.inspect;

        Box::pin(async move {
            // Copy `request.event_listener` so it can be used after ownership
            // of `request` is moved into `next.call()`. This is a cheap operation
            // since `EventListener` is a wrapper around a function pointer.
            let event_listener = *request.event_listener();
            // Clone `request.state` so it can be used after ownership of
            // `request` is moved into `next.call()`.
            let state = Arc::clone(request.state());

            next.call(request).await.or_else(|error| {
                // Pass a reference to `error` and `state` to the `inspect`
                // function. This allows the error to be logged or reported
                // based on the needs of the application.
                inspect(&error, &state);

                // Convert the error to a response and return `Ok`.
                Ok(respond_with_error(&event_listener, &state, error))
            })
        })
    }
}
