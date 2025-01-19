use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::next::Next;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

pub(crate) type ArcMiddleware<T> = Arc<dyn Middleware<T>>;
pub(crate) type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// The output of the `Future` returned from middleware.
///
pub type Result = std::result::Result<Response, Error>;

pub trait Middleware<T>: Send + Sync {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture<Result>;
}

impl<T, F, M> Middleware<T> for M
where
    M: Fn(Request<T>, Next<T>) -> F + Send + Sync,
    F: Future<Output = Result> + Send + 'static,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture<Result> {
        Box::pin(self(request, next))
    }
}
