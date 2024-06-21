use std::{future::Future, pin::Pin, sync::Arc};

use super::Next;
use crate::{IntoResponse, Request, Response, Result};

pub(crate) type DynMiddleware = Pin<Arc<dyn Middleware>>;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Middleware: Send + Sync + 'static {
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>>;
}

impl<F, T> Middleware for T
where
    F::Output: IntoResponse,
    F: Future + Send + 'static,
    T: Fn(Request, Next) -> F + Send + Sync + 'static,
{
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        let future = self(request, next);
        Box::pin(async { future.await.into_response() })
    }
}
