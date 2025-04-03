use std::sync::Arc;

use super::middleware::Middleware;
use super::next::Next;
use crate::error::Error;
use crate::request::Request;

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to inspect the error to another error. Think of this as a
/// [`Result::inspect_err`] for middleware.
///
pub fn inspect<T, F>(inspect: F) -> impl Middleware<T>
where
    F: Fn(&Error) + Copy + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    move |request: Request<T>, next: Next<T>| {
        let future = next.call(request);

        async move {
            future.await.or_else(|error| {
                inspect(&error);
                Ok(error.into())
            })
        }
    }
}

/// Similar to [`inspect`], but includes an owned arc to the state argument
/// that was passed to [`via::app`](crate::app::app) as the first argument
/// to the provided closure.
///
pub fn inspect_with_state<T, F>(inspect: F) -> impl Middleware<T>
where
    F: Fn(Arc<T>, &Error) + Copy + Send + Sync + 'static,
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
                inspect(state, &error);
                Ok(error.into())
            })
        }
    }
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to map the error to another error. Think of this as a
/// `Result::map` for middleware.
///
pub fn map<T, F>(map: F) -> impl Middleware<T>
where
    F: Fn(Error) -> Error + Copy + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    move |request: Request<T>, next: Next<T>| {
        let future = next.call(request);

        async move {
            // Await the future. If it resolves to a `Result::Err`, call the
            // provided map function with the error and a reference to the global
            // application state. Then generate a response from the returned
            // error.
            future.await.or_else(|error| Ok(map(error).into()))
        }
    }
}

/// Similar to [`map`], but includes an owned arc to the state argument that
/// was passed to [`via::app`](crate::app::app) as the first argument to the
/// provided closure.
///
pub fn map_with_state<T, F>(map: F) -> impl Middleware<T>
where
    F: Fn(Arc<T>, Error) -> Error + Copy + Send + Sync + 'static,
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
            future.await.or_else(|error| Ok(map(state, error).into()))
        }
    }
}
