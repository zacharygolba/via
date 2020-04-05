use super::context::Context;
use crate::{BoxFuture, Respond, Result};
use std::{collections::VecDeque, future::Future, sync::Arc};

pub(crate) type DynMiddleware = Arc<dyn Middleware>;

pub trait Middleware: Send + Sync + 'static {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result>;
}

pub struct Next {
    stack: VecDeque<DynMiddleware>,
}

impl<F, T> Middleware for T
where
    F::Output: Respond,
    F: Future + Send + 'static,
    T: Fn(Context, Next) -> F + Send + Sync + 'static,
{
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        let future = self(context, next);
        Box::pin(async { future.await.respond() })
    }
}

impl Next {
    pub(crate) fn new<'a>(stack: impl Iterator<Item = &'a DynMiddleware>) -> Self {
        Next {
            stack: stack.cloned().collect(),
        }
    }

    pub fn call(mut self, context: Context) -> BoxFuture<Result> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(context, self)
        } else {
            Box::pin(async { "Not Found".status(404).respond() })
        }
    }
}
