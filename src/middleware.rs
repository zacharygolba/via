use std::future::Future;
use std::pin::Pin;

use super::next::Next;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

/// The output of the `Future` returned from middleware.
///
pub type Result<T = Response> = std::result::Result<T, Error>;
pub type BoxFuture<T = Result> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub trait Middleware<T>: Send + Sync {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture;
}

impl<M, T, F> Middleware<T> for M
where
    M: Fn(Request<T>, Next<T>) -> F + Send + Sync,
    F: Future<Output = Result> + Send + 'static,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        Box::pin(self(request, next))
    }
}
