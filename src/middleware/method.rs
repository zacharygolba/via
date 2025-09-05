use http::Method;

use super::{Filter, Middleware, Predicate, filter};
use crate::request::Request;

pub fn connect<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::CONNECT, middleware)
}

pub fn delete<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::DELETE, middleware)
}

pub fn get<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::GET, middleware)
}

pub fn head<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::HEAD, middleware)
}

pub fn options<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::OPTIONS, middleware)
}

pub fn patch<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::PATCH, middleware)
}

pub fn post<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::POST, middleware)
}

pub fn put<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::PUT, middleware)
}

pub fn trace<M, T>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::TRACE, middleware)
}

impl<T> Predicate<T> for Method {
    #[inline]
    fn matches(&self, request: &Request<T>) -> bool {
        self == request.method()
    }
}
