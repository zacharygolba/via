use std::{future::Future, pin::Pin, sync::Arc};

use super::Next;
use crate::{Context, IntoResponse, Result};

pub(crate) type DynMiddleware = Pin<Arc<dyn Middleware>>;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Middleware: Send + Sync + 'static {
    fn call(self: Pin<&Self>, context: Context, next: Next) -> BoxFuture<Result>;
}

impl<F, T> Middleware for T
where
    F::Output: IntoResponse,
    F: Future + Send + 'static,
    T: Fn(Context, Next) -> F + Send + Sync + 'static,
{
    fn call(self: Pin<&Self>, context: Context, next: Next) -> BoxFuture<Result> {
        let future = self(context, next);
        Box::pin(async { future.await.into_response() })
    }
}
