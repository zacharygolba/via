use http::Method;

use crate::{BoxFuture, Context, Middleware, Next, Result};

pub struct MethodFilter<T: Middleware> {
    middleware: T,
    predicate: Method,
}

impl<T: Middleware> MethodFilter<T> {
    pub(crate) fn new(predicate: Method, middleware: T) -> Self {
        MethodFilter {
            middleware,
            predicate,
        }
    }
}

impl<T: Middleware> Middleware for MethodFilter<T> {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        if self.predicate == context.method() {
            self.middleware.call(context, next)
        } else {
            next.call(context)
        }
    }
}
