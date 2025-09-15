use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::{BoxFuture, Middleware, Next, Request, Response};

pub struct Resource<T> {
    allowed: String,
    methods: Vec<(Method, Box<dyn Middleware<T>>)>,
    #[allow(clippy::type_complexity)]
    or_else: Option<Box<dyn Fn(Request<T>, Next<T>) -> BoxFuture + Send + Sync>>,
}

pub fn connect<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::CONNECT, middleware)
}

pub fn delete<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::DELETE, middleware)
}

pub fn get<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::GET, middleware)
}

pub fn head<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::HEAD, middleware)
}

pub fn options<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::OPTIONS, middleware)
}

pub fn patch<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::PATCH, middleware)
}

pub fn post<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::POST, middleware)
}

pub fn put<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::PUT, middleware)
}

pub fn trace<State, T>(middleware: T) -> Resource<State>
where
    T: Middleware<State> + 'static,
{
    Resource::new(Method::TRACE, middleware)
}

impl<State> Resource<State> {
    pub fn and(mut self, other: Resource<State>) -> Self {
        let allowed = &mut self.allowed;

        for (method, _) in &other.methods {
            allowed.push_str(", ");
            allowed.push_str(method.as_str());
        }

        self.methods.extend(other.methods);
        self
    }

    pub fn or_else<F>(mut self, respond: F) -> Self
    where
        F: Fn(Request<State>, Next<State>) -> BoxFuture + Send + Sync + 'static,
    {
        self.or_else = Some(Box::new(respond));
        self
    }

    pub fn or_next(self) -> Self {
        self.or_else(|request, next| next.call(request))
    }
}

impl<State> Resource<State> {
    pub(crate) fn new<T>(method: Method, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        Self {
            allowed: method.as_str().to_owned(),
            methods: vec![(method, Box::new(middleware))],
            or_else: None,
        }
    }
}

impl<State> Middleware<State> for Resource<State> {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let method = request.method();

        for (allow, middleware) in &self.methods {
            if allow == method {
                return middleware.call(request, next);
            }
        }

        if let Some(or_else) = &self.or_else {
            or_else(request, next)
        } else {
            let mut response = Response::from(
                crate::error!(405, "Request method \"{}\" is not supported.", method).as_json(),
            );

            if let Ok(header) = HeaderValue::from_str(&self.allowed) {
                response.headers_mut().insert(ALLOW, header);
            }

            Box::pin(async { Ok(response) })
        }
    }
}
