use http::Method;

use crate::{BoxFuture, Middleware, Next, Request, Response, Result};

pub struct AllowMethod<T> {
    middleware: T,
    predicate: Method,
}

impl<T> AllowMethod<T> {
    pub(crate) fn new(predicate: Method, middleware: T) -> Self {
        Self {
            middleware,
            predicate,
        }
    }
}

impl<State, T> Middleware<State> for AllowMethod<T>
where
    T: Middleware<State>,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        if self.predicate == request.method() {
            self.middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
