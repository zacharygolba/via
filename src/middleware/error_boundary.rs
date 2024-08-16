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
    map: Arc<F>,
}

/// Works like `ErrorBoundary` but allows you to inspect the error before it is
/// converted into a response. This is useful for logging the error message
/// or reporting the error to a monitoring service.
pub struct InspectErrorBoundary<F> {
    inspect: Arc<F>,
}

/// Performs the conversion of an `Error` into a `Response`. If the conversion
/// fails, a fallback response is generated and `event_listener` is notified of
/// the error that prevented the response from being generated.
fn respond_with_error(event_listener: &EventListener, error: Error) -> Response {
    match error.try_into_response() {
        Ok(response) => response,
        Err((fallback_response, convert_error)) => {
            // Notify `event_listener` of the error that prevented the response
            // from being generated. This allows the error to be reported or
            // handled in the most suitable way for the application's needs.
            event_listener.call(Event::UncaughtError(&convert_error));

            // Return the fallback response that contains `error` as plain text.
            fallback_response
        }
    }
}

impl ErrorBoundary {
    pub fn inspect<F>(inspect: F) -> InspectErrorBoundary<F>
    where
        F: Fn(&Error) + Send + Sync + 'static,
    {
        InspectErrorBoundary {
            inspect: Arc::new(inspect),
        }
    }

    pub fn map<F>(map: F) -> MapErrorBoundary<F>
    where
        F: Fn(Error) -> Error + Send + Sync + 'static,
    {
        MapErrorBoundary { map: Arc::new(map) }
    }
}

impl<State> Middleware<State> for ErrorBoundary
where
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let event_listener = Arc::clone(request.event_listener());

        Box::pin(async move {
            next.call(request).await.or_else(|error| {
                // Convert the error to a response and return `Ok`.
                Ok(respond_with_error(&event_listener, error))
            })
        })
    }
}

impl<State, F> Middleware<State> for MapErrorBoundary<F>
where
    F: Fn(Error) -> Error + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let event_listener = Arc::clone(request.event_listener());
        let map = Arc::clone(&self.map);

        Box::pin(async move {
            next.call(request).await.or_else(|error| {
                // Apply the `map` function to `error`.
                let mapped = map(error);

                // Convert the mapped error to a response and return `Ok`.
                Ok(respond_with_error(&event_listener, mapped))
            })
        })
    }
}

impl<State, F> Middleware<State> for InspectErrorBoundary<F>
where
    F: Fn(&Error) + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let event_listener = Arc::clone(request.event_listener());
        let inspect = Arc::clone(&self.inspect);

        Box::pin(async move {
            next.call(request).await.or_else(|error| {
                // Call the `inspect` function before ownership of `error` is
                // moved into `respond_with_error`.
                inspect(&error);

                // Convert the error to a response and return `Ok`.
                Ok(respond_with_error(&event_listener, error))
            })
        })
    }
}
