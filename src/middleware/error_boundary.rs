use std::{pin::Pin, sync::Arc};

use crate::{
    event::{Event, EventListener},
    BoxFuture, Error, Middleware, Next, Request, Response, Result,
};

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

impl Middleware for ErrorBoundary {
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        Box::pin(async {
            let event_listener = request.get::<EventListener>().cloned()?;

            next.call(request).await.or_else(|error| {
                Ok(error.into_infallible_response(|error| {
                    event_listener.call(Event::UncaughtError(&error))
                }))
            })
        })
    }
}

impl<F> Middleware for MapErrorBoundary<F>
where
    F: Fn(Error) -> Error + Send + Sync + 'static,
{
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        let map = Arc::clone(&self.map);

        Box::pin(async move {
            let event_listener = request.get::<EventListener>().cloned()?;

            next.call(request).await.or_else(|error| {
                Ok(map(error).into_infallible_response(|error| {
                    event_listener.call(Event::UncaughtError(&error))
                }))
            })
        })
    }
}

impl<F> Middleware for InspectErrorBoundary<F>
where
    F: Fn(&Error) + Send + Sync + 'static,
{
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        let inspect = Arc::clone(&self.inspect);

        Box::pin(async move {
            let event_listener = request.get::<EventListener>().cloned()?;

            next.call(request).await.or_else(|error| {
                inspect(&error);
                Ok(error.into_infallible_response(|error| {
                    event_listener.call(Event::UncaughtError(&error));
                }))
            })
        })
    }
}
