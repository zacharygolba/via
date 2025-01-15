use super::middleware::Middleware;
use super::next::Next;
use crate::{Error, Request, Response};

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to inspect the error to another error. Think of this as a
/// [`Result::inspect_err`] for middleware.
///
pub fn catch<T, F>(inspect: F) -> impl Middleware<T>
where
    F: Fn(&T, &Error) + Copy + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    move |request: Request<T>, next: Next<T>| {
        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        // Call the next middleware to get a future that will resolve to a
        // response.
        let future = next.call(request);

        async move {
            // Await the future. If it resolves to a `Result::Err` call the
            // provided inspect fn with a reference to the contained error and
            // the global application state.
            future.await.or_else(|error| {
                inspect(&state, &error);
                Ok(error.into())
            })
        }
    }
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to map the error to another error. Think of this as a
/// `Result::map` for middleware.
///
/// Middleware that is upstream from a `MapErrorBoundary` will continue to
/// execute as usual since the error returned from the provided `map` function
/// is eagerly converted into a response.
///
pub fn map<T, F>(map: F) -> impl Middleware<T>
where
    F: Fn(&T, Error) -> Error + Copy + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    move |request: Request<T>, next: Next<T>| {
        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        // Call the next middleware to get a future that will resolve to a
        // response.
        let future = next.call(request);

        async move {
            // Await the future. If it resolves to a `Result::Err`, call the
            // provided map function with the error and a reference to the global
            // application state. Then generate a response from the returned
            // error.
            future.await.or_else(|error| Ok(map(&state, error).into()))
        }
    }
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to map the error to another result. Think of this as a
/// `Result::or_else` for middleware.
///
/// Middleware that is upstream from a `OrElseErrorBoundary` will continue to
/// execute as usual since the result returned from the provided `or_else`
/// function is eagerly converted into a response.
///
pub fn or_else<T, F>(or_else: F) -> impl Middleware<T>
where
    F: Fn(&T, Error) -> Result<Response, Error> + Copy + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    move |request: Request<T>, next: Next<T>| {
        // Clone `request.state` so it can be used after ownership of `request`
        // is transfered to `next.call()`.
        let state = request.state().clone();

        // Call the next middleware to get a future that will resolve to a
        // response.
        let future = next.call(request);

        async move {
            // Await the future. If it resolves to a `Result::Err`, call the p
            // provided or_else function with the error and a reference to the
            // global application state.
            future.await.or_else(|error| or_else(&state, error))
        }
    }
}
