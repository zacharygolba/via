use std::future::Future;
use std::pin::Pin;

use super::next::Next;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

/// The output of the `Future` returned from middleware.
///
pub type Result = std::result::Result<Response, Error>;
pub type BoxFuture = Pin<Box<dyn Future<Output = Result> + Send + 'static>>;

pub trait Middleware<T>: Send + Sync {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture;
}

impl<T, F, M> Middleware<T> for M
where
    M: Fn(Request<T>, Next<T>) -> F + Send + Sync,
    F: Future<Output = Result> + Send + 'static,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        Box::pin(self(request, next))
    }
}
