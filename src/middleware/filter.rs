use http::Method;

use crate::{BoxFuture, Context, Middleware, Next, Result};

pub struct MethodFilter<T: Middleware> {
    middleware: T,
    predicate: Method,
}

pub fn method<T: Middleware>(predicate: Method) -> impl FnOnce(T) -> MethodFilter<T> {
    move |middleware| MethodFilter {
        middleware,
        predicate,
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
