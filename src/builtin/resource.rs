use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::{BoxFuture, Middleware, Next, Request, Response};

pub struct Resource<T> {
    allow: String,
    accept: Vec<(Method, Box<dyn Middleware<T>>)>,
    or_else: Option<Box<dyn Fn(Request<T>, Next<T>) -> BoxFuture + Send + Sync>>,
}

pub fn connect<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::CONNECT, middleware)
}

pub fn delete<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::DELETE, middleware)
}

pub fn get<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::GET, middleware)
}

pub fn head<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::HEAD, middleware)
}

pub fn options<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::OPTIONS, middleware)
}

pub fn patch<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::PATCH, middleware)
}

pub fn post<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::POST, middleware)
}

pub fn put<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::PUT, middleware)
}

pub fn trace<M, T>(middleware: M) -> Resource<T>
where
    M: Middleware<T> + 'static,
{
    Resource::new(Method::TRACE, middleware)
}

impl<T> Resource<T> {
    pub fn and(mut self, other: Resource<T>) -> Self {
        let allow = &mut self.allow;
        let accept = &mut self.accept;

        for (method, _) in &other.accept {
            allow.push_str(", ");
            allow.push_str(method.as_str());
        }

        accept.extend(other.accept);
        self
    }

    pub fn or_else<F>(mut self, respond: F) -> Self
    where
        F: Fn(Request<T>, Next<T>) -> BoxFuture + Send + Sync + 'static,
    {
        self.or_else = Some(Box::new(respond));
        self
    }

    pub fn or_next(self) -> Self {
        self.or_else(|request, next| next.call(request))
    }
}

impl<T> Resource<T> {
    pub(crate) fn new<M>(method: Method, middleware: M) -> Self
    where
        M: Middleware<T> + 'static,
    {
        Self {
            allow: method.as_str().to_owned(),
            accept: vec![(method, Box::new(middleware))],
            or_else: None,
        }
    }
}

impl<T> Middleware<T> for Resource<T> {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        let method = request.method();

        for (accept, middleware) in &self.accept {
            if accept == method {
                return middleware.call(request, next);
            }
        }

        if let Some(or_else) = &self.or_else {
            or_else(request, next)
        } else {
            let mut response = Response::from(
                crate::error!(405, "Request method \"{}\" is not supported.", method).as_json(),
            );

            if let Ok(header) = HeaderValue::from_str(&self.allow) {
                response.headers_mut().insert(ALLOW, header);
            }

            Box::pin(async { Ok(response) })
        }
    }
}
