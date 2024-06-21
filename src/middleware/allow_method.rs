use http::Method;
use std::pin::Pin;

use crate::{BoxFuture, Middleware, Next, Request, Response, Result};

pub struct AllowMethod<T: Middleware> {
    middleware: Pin<Box<T>>,
    predicate: Method,
}

impl<T: Middleware> AllowMethod<T> {
    pub(crate) fn new(predicate: Method, middleware: T) -> Self {
        AllowMethod {
            middleware: Box::pin(middleware),
            predicate,
        }
    }
}

impl<T: Middleware> Middleware for AllowMethod<T> {
    fn call(self: Pin<&Self>, request: Request, next: Next) -> BoxFuture<Result<Response>> {
        if self.predicate == request.method() {
            self.middleware.as_ref().call(request, next)
        } else {
            Box::pin(next.call(request))
        }
    }
}
