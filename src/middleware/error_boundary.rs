use crate::middleware::BoxFuture;
use crate::{Error, Middleware, Next, Request, Response, Result};

/// Middleware that catches errors that occur in downstream middleware and
/// converts the error into a response. Middleware that is upstream from a
/// `ErrorBoundary` will continue to as usual.
pub struct ErrorBoundary;

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to inspect the error to another error. Think of this as a
/// [`Result::inspect_err`] for middleware.
pub struct InspectErrorBoundary<F> {
    inspect: F,
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to map the error to another error. Think of this as a
/// `Result::map` for middleware.
///
/// Middleware that is upstream from a `MapErrorBoundary` will continue to
/// execute as usual since the error returned from the provided `map` function
/// is eagerly converted into a response.
pub struct MapErrorBoundary<F> {
    map: F,
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to map the error to another result. Think of this as a
/// `Result::or_else` for middleware.
///
/// Middleware that is upstream from a `OrElseErrorBoundary` will continue to
/// execute as usual since the result returned from the provided `or_else`
/// function is eagerly converted into a response.
pub struct OrElseErrorBoundary<F> {
    or_else: F,
}

impl ErrorBoundary {
    pub fn inspect<State, F>(inspect: F) -> InspectErrorBoundary<F>
    where
        F: Fn(&Error, &State) + Copy + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        InspectErrorBoundary { inspect }
    }

    pub fn map<State, F>(map: F) -> MapErrorBoundary<F>
    where
        F: Fn(Error, &State) -> Error + Copy + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        MapErrorBoundary { map }
    }

    pub fn or_else<State, F>(or_else: F) -> OrElseErrorBoundary<F>
    where
        F: Fn(Error, &State) -> Result<Response, Error> + Copy + Send + Sync + 'static,
        State: Send + Sync + 'static,
    {
        OrElseErrorBoundary { or_else }
    }
}

impl<State> Middleware<State> for ErrorBoundary
where
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        Box::pin(async {
            // Yield control to the next middleware in the stack.
            let result = next.call(request).await;

            // Convert the error into a response and return.
            Ok(result.unwrap_or_else(|error| error.into_response()))
        })
    }
}

impl<State, F> Middleware<State> for InspectErrorBoundary<F>
where
    F: Fn(&Error, &State) + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        // Copy the `inspect` function so it can be moved in the async block.
        let inspect = self.inspect;
        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        Box::pin(async move {
            // Yield control to the next middleware in the stack.
            let result = next.call(request).await;

            result.inspect_err(|error| {
                // Pass a reference to `error` and `state` to the `inspect`
                // function. This allows the error to be logged or reported
                // based on the needs of the application.
                inspect(error, &state);
            })
        })
    }
}

impl<State, F> Middleware<State> for MapErrorBoundary<F>
where
    F: Fn(Error, &State) -> Error + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        // Copy the `map` function so it can be moved in the async block.
        let map = self.map;
        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        Box::pin(async move {
            // Yield control to the next middleware in the stack.
            let result = next.call(request).await;

            result.or_else(|error| {
                // Apply the `map` function to `error`. This allows the error
                // to be configured to use a different response format, filter
                // out sensitive information from leaking into the response
                // body, etc.
                let error = map(error, &state);

                // Convert the error into a response and return.
                Ok(error.into_response())
            })
        })
    }
}

impl<State, F> Middleware<State> for OrElseErrorBoundary<F>
where
    F: Fn(Error, &State) -> Result<Response, Error> + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        // Copy the `or_else` function so it can be moved in the async block.
        let or_else = self.or_else;
        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        Box::pin(async move {
            // Yield control to the next middleware in the stack.
            let result = next.call(request).await;

            result.or_else(|error| {
                // Apply the `or_else` function to the output of `next.call()`.
                // This allows the error to be handled in a way that may
                // produce a different response or error.
                or_else(error, &state)
            })
        })
    }
}
