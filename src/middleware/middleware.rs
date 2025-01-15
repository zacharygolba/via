use std::{future::Future, pin::Pin, sync::Arc};

use super::Next;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

pub(crate) type ArcMiddleware<T> = Arc<dyn Middleware<T>>;
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Middleware<T>: Send + Sync {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture<Result<Response, Error>>;
}

impl<T, F, M> Middleware<T> for M
where
    M: Fn(Request<T>, Next<T>) -> F + Send + Sync,
    F: Future<Output = Result<Response, Error>> + Send + 'static,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture<Result<Response, Error>> {
        Box::pin(self(request, next))
    }
}
