use http::Method;

use crate::{BoxFuture, Context, Middleware, Next, Result};

pub struct AllowMethod<T: Middleware> {
    middleware: T,
    predicate: Method,
}

impl<T: Middleware> AllowMethod<T> {
    pub(crate) fn new(predicate: Method, middleware: T) -> Self {
        AllowMethod {
            middleware,
            predicate,
        }
    }
}

impl<T: Middleware> Middleware for AllowMethod<T> {
    fn call(&self, context: Context, next: Next) -> BoxFuture<Result> {
        if self.predicate == context.method() {
            self.middleware.call(context, next)
        } else {
            next.call(context)
        }
    }
}
