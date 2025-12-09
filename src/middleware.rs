use std::future::Future;
use std::pin::Pin;

use super::next::Next;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

/// An alias for the pin
/// [`Box<dyn Future>`]
/// returned by
/// [middleware](Middleware::call).
///
pub type BoxFuture<T = Result> = Pin<Box<dyn Future<Output = T> + Send>>;

/// An alias for results that uses the [`Error`] struct defined in this crate.
///
pub type Result<T = Response> = std::result::Result<T, Error>;

pub trait Middleware<State>: Send + Sync {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture;
}

impl<State, F, R> Middleware<State> for F
where
    F: Fn(Request<State>, Next<State>) -> R + Send + Sync,
    R: Future<Output = Result> + Send + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        Box::pin(self(request, next))
    }
}
