use http::Method;

use super::Middleware;
use crate::request::Request;

pub fn connect<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::CONNECT, middleware)
}

pub fn delete<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::DELETE, middleware)
}

pub fn get<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::GET, middleware)
}

pub fn head<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::HEAD, middleware)
}

pub fn options<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::OPTIONS, middleware)
}

pub fn patch<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::PATCH, middleware)
}

pub fn post<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::POST, middleware)
}

pub fn put<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::PUT, middleware)
}

pub fn trace<T>(middleware: impl Middleware<T>) -> impl Middleware<T> {
    accept_method(Method::TRACE, middleware)
}

fn accept_method<T>(method: Method, middleware: impl Middleware<T>) -> impl Middleware<T> {
    move |request: Request<T>, next| {
        if request.method() == method {
            middleware.call(request, next)
        } else {
            next.call(request)
        }
    }
}
