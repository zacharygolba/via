use http::Method;

use crate::{BoxFuture, Context, Middleware, Next, Result};

pub struct AllowMethod<T: Middleware> {
    middleware: T,
    predicate: Method,
}

pub fn connect<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::CONNECT, middleware)
}

pub fn delete<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::DELETE, middleware)
}

pub fn get<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::GET, middleware)
}

pub fn head<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::HEAD, middleware)
}

pub fn options<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::OPTIONS, middleware)
}

pub fn patch<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::PATCH, middleware)
}

pub fn post<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::POST, middleware)
}

pub fn put<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::PUT, middleware)
}

pub fn trace<T: Middleware>(middleware: T) -> AllowMethod<T> {
    AllowMethod::new(Method::TRACE, middleware)
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
