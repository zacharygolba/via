use http::Method;

use super::Middleware;
use crate::request::Request;

pub fn connect<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::CONNECT, middleware)
}

pub fn delete<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::DELETE, middleware)
}

pub fn get<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::GET, middleware)
}

pub fn head<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::HEAD, middleware)
}

pub fn options<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::OPTIONS, middleware)
}

pub fn patch<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::PATCH, middleware)
}

pub fn post<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::POST, middleware)
}

pub fn put<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::PUT, middleware)
}

pub fn trace<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    filter(Method::TRACE, middleware)
}

fn filter<T>(method: Method, middleware: impl Middleware<T>) -> impl Middleware<T> {
    move |request: Request<T>, next| {
        if request.method() == method {
            middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
