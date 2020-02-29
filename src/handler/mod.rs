pub mod context;
pub mod respond;

use crate::{http::StatusCode, Result};
use std::{collections::VecDeque, pin::Pin};

pub use self::{
    context::Context,
    respond::{Respond, Response},
};

pub(crate) type DynHandler = Box<dyn Handler>;
pub type Future = Pin<Box<dyn std::future::Future<Output = Result> + Send>>;

pub trait Handler: Send + Sync + 'static {
    fn call<'a>(&'a self, context: Context, next: Next<'a>) -> Future;
}

pub struct Next<'a> {
    stack: VecDeque<&'a DynHandler>,
}

impl<F, T> Handler for T
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
        T: Iterator<Item = &'a DynHandler>,
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
            Box::pin(async { StatusCode::NOT_FOUND.respond() })
        }
    }
}
