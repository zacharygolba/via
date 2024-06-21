use std::{pin::Pin, sync::Arc};

use crate::{BoxFuture, Error, Middleware, Next, Request, Response, Result};

pub struct ErrorBoundary;

pub struct MapErrorBoundary<F> {
    map: Arc<F>,
}

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
            match next.call(request).await {
                result @ Ok(_) => result,
                Err(error) => Ok(error.into_infallible_response()),
            }
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
            match next.call(request).await {
                result @ Ok(_) => result,
                Err(error) => Ok(map(error).into_infallible_response()),
            }
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
            match next.call(request).await {
                result @ Ok(_) => result,
                Err(error) => {
                    inspect(&error);
                    Ok(error.into_infallible_response())
                }
            }
        })
    }
}
