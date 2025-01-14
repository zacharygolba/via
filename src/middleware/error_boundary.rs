use super::middleware::{BoxFuture, Middleware};
use super::next::Next;
use crate::{Error, Request, Response};

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to inspect the error to another error. Think of this as a
/// [`Result::inspect_err`] for middleware.
pub struct CatchErrorBoundary<F> {
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

pub fn catch<State, F>(inspect: F) -> CatchErrorBoundary<F>
where
    F: Fn(&Error, &State) + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    CatchErrorBoundary { inspect }
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

impl<State, F> Middleware<State> for CatchErrorBoundary<F>
where
    F: Fn(&Error, &State) + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(
        &self,
        request: Request<State>,
        next: Next<State>,
    ) -> BoxFuture<Result<Response, Error>> {
        // Copy the `inspect` function so it can be moved in the async block.
        let inspect = self.inspect;

        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        // Call the next middleware to get a future that will resolve to a
        // response.
        let future = next.call(request);

        Box::pin(async move {
            // Await the future. If it resolves to a `Result::Err` call the
            // provided inspect fn with a reference to the contained error and
            // the global application state.
            future.await.or_else(|error| {
                inspect(&error, &state);
                Ok(error.into())
            })
        })
    }
}

impl<State, F> Middleware<State> for MapErrorBoundary<F>
where
    F: Fn(Error, &State) -> Error + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(
        &self,
        request: Request<State>,
        next: Next<State>,
    ) -> BoxFuture<Result<Response, Error>> {
        // Copy the `map` function so it can be moved in the async block.
        let map = self.map;

        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        // Call the next middleware to get a future that will resolve to a
        // response.
        let future = next.call(request);

        Box::pin(async move {
            // Await the future. If it resolves to a `Result::Err`, call the
            // provided map function with the error and a reference to the global
            // application state. Then generate a response from the returned
            // error.
            future.await.or_else(|error| Ok(map(error, &state).into()))
        })
    }
}

impl<State, F> Middleware<State> for OrElseErrorBoundary<F>
where
    F: Fn(Error, &State) -> Result<Response, Error> + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(
        &self,
        request: Request<State>,
        next: Next<State>,
    ) -> BoxFuture<Result<Response, Error>> {
        // Copy the `or_else` function so it can be moved in the async block.
        let or_else = self.or_else;

        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        // Call the next middleware to get a future that will resolve to a
        // response.
        let future = next.call(request);

        Box::pin(async move {
            // Await the future. If it resolves to a `Result::Err`, call the p
            // provided or_else function with the error and a reference to the
            // global application state.
            future.await.or_else(|error| or_else(error, &state))
        })
    }
}
