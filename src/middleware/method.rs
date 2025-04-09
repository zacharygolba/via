use http::Method;

use super::{filter, Filter, Middleware, Predicate};
use crate::request::Request;

pub fn connect<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::CONNECT, middleware)
}

pub fn delete<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::DELETE, middleware)
}

pub fn get<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::GET, middleware)
}

pub fn head<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::HEAD, middleware)
}

pub fn options<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::OPTIONS, middleware)
}

pub fn patch<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::PATCH, middleware)
}

pub fn post<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::POST, middleware)
}

pub fn put<T, M>(middleware: M) -> Filter<Method, M>
where
    M: Middleware<T>,
{
    filter(Method::PUT, middleware)
}

pub fn trace<T, M>(middleware: M) -> Filter<Method, M>
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
