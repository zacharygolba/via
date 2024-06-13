pub mod allow_method;

use std::{collections::VecDeque, future::Future, pin::Pin, sync::Arc};

use crate::{Context, IntoResponse, Response, Result};

pub type BoxFuture<T> = futures::future::BoxFuture<'static, T>;
pub(crate) type DynMiddleware = Pin<Arc<dyn Middleware>>;

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
            Box::pin(async { Response::text("Not Found").status(404).end() })
        }
    }
}
