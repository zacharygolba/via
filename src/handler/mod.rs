mod context;
pub mod respond;

use crate::{BoxFuture, Result};
use std::{collections::VecDeque, sync::Arc};

pub use self::{
    context::*,
    respond::{Respond, Response},
};

pub(crate) type ArcMiddleware = Arc<dyn Middleware>;

pub trait Middleware: Send + Sync + 'static {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result>;
}

pub struct Next {
    stack: VecDeque<ArcMiddleware>,
}

impl<F, T> Middleware for T
where
    F::Output: Respond,
    F: std::future::Future + Send + 'static,
    T: Fn(Context, Next) -> F + Send + Sync + 'static,
{
    #[inline]
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        let future = self(context, next);
        Box::pin(async { future.await.respond() })
    }
}

impl Next {
    #[inline]
    pub(crate) fn new<'a, I>(stack: I) -> Next
    where
        I: Iterator<Item = &'a ArcMiddleware>,
    {
        Next {
            stack: stack.cloned().collect(),
        }
    }

    #[inline]
    pub fn call(mut self, context: Context) -> BoxFuture<Result> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(context, self)
        } else {
            Box::pin(async { "Not Found".status(404).respond() })
        }
    }
}
