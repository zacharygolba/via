#![allow(clippy::module_inception)]

mod app;
mod error;
mod event;
mod router;

pub mod body;
pub mod middleware;
pub mod request;
pub mod response;

pub use http;

pub use crate::{
    app::{app, App},
    error::{Error, Result},
    event::Event,
    middleware::{ErrorBoundary, Middleware, Next},
    request::Request,
    response::Response,
    router::Endpoint,
};

use crate::{
    http::Method,
    middleware::{AllowMethod, BoxFuture},
};

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
