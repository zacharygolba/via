use http::Method;

use super::{BoxFuture, Middleware, Next};
use crate::{Error, Request, Response};

pub struct AllowMethod<T> {
    middleware: T,
    predicate: Method,
}

pub fn connect<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::CONNECT, middleware)
}

pub fn delete<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::DELETE, middleware)
}

pub fn get<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::GET, middleware)
}

pub fn head<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::HEAD, middleware)
}

pub fn options<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::OPTIONS, middleware)
}

pub fn patch<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::PATCH, middleware)
}

pub fn post<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::POST, middleware)
}

pub fn put<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::PUT, middleware)
}

pub fn trace<State, T>(middleware: T) -> AllowMethod<T>
where
    T: Middleware<State>,
{
    AllowMethod::new(Method::TRACE, middleware)
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
    fn call(
        &self,
        request: Request<State>,
        next: Next<State>,
    ) -> BoxFuture<Result<Response, Error>> {
        if self.predicate == request.method() {
            self.middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
