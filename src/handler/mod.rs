mod context;
pub mod respond;

use crate::Result;
use std::{collections::VecDeque, pin::Pin};

pub use self::{
    context::*,
    respond::{Respond, Response},
};

pub(crate) type DynMiddleware = Box<dyn Middleware>;
#[doc(hidden)]
pub type Future = Pin<Box<dyn std::future::Future<Output = Result> + Send>>;

pub trait Middleware: Send + Sync + 'static {
    fn call<'a>(&'a self, context: Context, next: Next<'a>) -> Future;
}

pub struct Next<'a> {
    stack: VecDeque<&'a DynMiddleware>,
}

impl<F, T> Middleware for T
where
    F::Output: Respond,
    F: std::future::Future + Send + 'static,
    T: for<'a> Fn(Context, Next<'a>) -> F + Send + Sync + 'static,
{
    #[inline]
    fn call(&self, context: Context, next: Next) -> Future {
        let future = self(context, next);
        Box::pin(async { future.await.respond() })
    }
}

impl<'a> Next<'a> {
    #[inline]
    pub(crate) fn new<T>(stack: T) -> Next<'a>
    where
        T: Iterator<Item = &'a DynMiddleware>,
    {
        Next {
            stack: stack.collect(),
        }
    }

    #[inline]
    pub fn call(mut self, context: Context) -> Future {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(context, self)
        } else {
            Box::pin(async { "Not Found".status(404).respond() })
        }
    }
}
