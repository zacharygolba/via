use http::Method;

use crate::{BoxFuture, Middleware, Next, Request, Response, Result};

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
    fn call(&self, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        if self.predicate == request.method() {
            self.middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
