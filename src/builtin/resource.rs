use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::{BoxFuture, Middleware, Next, Request, Response};

pub struct Resource<T> {
    allowed: String,
    methods: Vec<(Method, Box<dyn Middleware<T>>)>,
    #[allow(clippy::type_complexity)]
    or_else: Option<Box<dyn Fn(Request<T>, Next<T>) -> BoxFuture + Send + Sync>>,
}

pub fn connect<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::CONNECT, middleware)
}

pub fn delete<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::DELETE, middleware)
}

pub fn get<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::GET, middleware)
}

pub fn head<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::HEAD, middleware)
}

pub fn options<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::OPTIONS, middleware)
}

pub fn patch<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::PATCH, middleware)
}

pub fn post<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::POST, middleware)
}

pub fn put<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::PUT, middleware)
}

pub fn trace<T>(middleware: impl Middleware<T> + 'static) -> Resource<T> {
    Resource::new(Method::TRACE, middleware)
}

impl<T> Resource<T> {
    pub fn and(mut self, other: Resource<T>) -> Self {
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
            allowed: method.as_str().to_owned(),
            methods: vec![(method, Box::new(middleware))],
            or_else: None,
        }
    }
}

impl<T> Middleware<T> for Resource<T> {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
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
