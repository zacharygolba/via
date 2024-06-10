pub mod context;
pub mod filter;

#[doc(inline)]
pub use self::context::Context;

use std::{collections::VecDeque, future::Future, sync::Arc};

use crate::{BoxFuture, IntoResponse, Result};

pub(crate) type DynMiddleware = Arc<dyn Middleware>;

pub trait Middleware: Send + Sync + 'static {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result>;
}

pub struct Next {
    stack: VecDeque<DynMiddleware>,
}

impl<F, T> Middleware for T
where
    F::Output: IntoResponse,
    F: Future + Send + 'static,
    T: Fn(Context, Next) -> F + Send + Sync + 'static,
{
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        let future = self(context, next);
        Box::pin(async { future.await.into_response() })
    }
}

impl Next {
    pub(crate) fn new(stack: VecDeque<DynMiddleware>) -> Self {
        Next { stack }
    }

    pub fn call(mut self, context: Context) -> BoxFuture<Result> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(context, self)
        } else {
            Box::pin(async { "Not Found".with_status(404).into_response() })
        }
    }
}
