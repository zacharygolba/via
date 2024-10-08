use std::{future::Future, pin::Pin, sync::Arc};

use super::Next;
use crate::{response::IntoResponse, Request, Response, Result};

pub(crate) type ArcMiddleware<State> = Arc<dyn Middleware<State>>;
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Middleware<State>: Send + Sync {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>>;
}

impl<State, T, F> Middleware<State> for T
where
    T: Fn(Request<State>, Next<State>) -> F + Send + Sync + 'static,
    F: Future + Send + 'static,
    F::Output: IntoResponse,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let future = self(request, next);
        Box::pin(async { future.await.into_response() })
    }
}
