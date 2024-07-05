use std::{future::Future, pin::Pin, sync::Arc};

use super::Next;
use crate::{Request, Response, Result};

pub(crate) type DynMiddleware<State> = Arc<dyn Middleware<State>>;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Middleware<State>: Send + Sync + 'static {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>>;
}

impl<State, T, F> Middleware<State> for T
where
    T: Fn(Request<State>, Next<State>) -> F + Send + Sync + 'static,
    F: Future<Output = Result<Response>> + Send + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        Box::pin(self(request, next))
    }
}
