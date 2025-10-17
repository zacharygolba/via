use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::middleware::{BoxFuture, Middleware};
use crate::{Next, Request, Response};

/// Middleware for routing based on the HTTP method of the request.
///
/// When a route has different behavior depending on the HTTP method, grouping
/// the various handlers into an [Allow] middleware can prevent the middleware
/// stack from reallocating during request routing and reduce per-method
/// atomic operations into a single operation.
///
pub struct Allow<State> {
    allowed: Vec<(Method, Box<dyn Middleware<State>>)>,
    or_next: bool,
}

/// Allow `CONNECT` requests to call the provided middleware.
///
pub fn connect<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::CONNECT, middleware)
}

/// Allow `DELETE` requests to call the provided middleware.
///
pub fn delete<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::DELETE, middleware)
}

/// Allow `GET` requests to call the provided middleware.
///
pub fn get<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::GET, middleware)
}

/// Allow `HEAD` requests to call the provided middleware.
///
pub fn head<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::HEAD, middleware)
}

/// Allow `OPTIONS` requests to call the provided middleware.
///
pub fn options<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::OPTIONS, middleware)
}

/// Allow `PATCH` requests to call the provided middleware.
///
pub fn patch<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::PATCH, middleware)
}

/// Allow `POST` requests to call the provided middleware.
///
pub fn post<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::POST, middleware)
}

/// Allow `PUT` requests to call the provided middleware.
///
pub fn put<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::PUT, middleware)
}

/// Allow `TRACE` requests to call the provided middleware.
///
pub fn trace<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::TRACE, middleware)
}

impl<State> Allow<State> {
    pub fn connect<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::CONNECT, Box::new(middleware)));
        self
    }

    pub fn delete<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::DELETE, Box::new(middleware)));
        self
    }

    pub fn get<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::GET, Box::new(middleware)));
        self
    }

    pub fn head<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::HEAD, Box::new(middleware)));
        self
    }

    pub fn options<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::OPTIONS, Box::new(middleware)));
        self
    }

    pub fn patch<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::PATCH, Box::new(middleware)));
        self
    }

    pub fn post<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::POST, Box::new(middleware)));
        self
    }

    pub fn put<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::PUT, Box::new(middleware)));
        self
    }

    pub fn trace<T>(mut self, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        self.allowed.push((Method::TRACE, Box::new(middleware)));
        self
    }

    /// Return a `405 Method Not Allowed` response if the request method is not
    /// supported.
    ///
    pub fn or_method_not_allowed(mut self) -> Self {
        self.or_next = false;
        self
    }
}

impl<State> Allow<State> {
    fn new(method: Method, middleware: impl Middleware<State> + 'static) -> Self {
        Self {
            allowed: vec![(method, Box::new(middleware))],
            or_next: true,
        }
    }

    fn allow_header(&self) -> Option<HeaderValue> {
        let allowed = self.allowed.iter().fold(None, |init, (method, _)| {
            Some(match init {
                Some(allowed) => allowed + ", " + method.as_str(),
                None => method.as_str().to_owned(),
            })
        })?;

        allowed.try_into().ok()
    }

    fn respond_to(&self, method: &Method) -> Option<&dyn Middleware<State>> {
        self.allowed.iter().find_map(|(allow, middleware)| {
            if method == allow {
                Some(middleware.as_ref())
            } else {
                None
            }
        })
    }
}

impl<State> Middleware<State> for Allow<State> {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        if let Some(middleware) = self.respond_to(request.method()) {
            middleware.call(request, next)
        } else if self.or_next {
            next.call(request)
        } else {
            let mut response = Response::from(crate::raise!(405));

            if let Some(value) = self.allow_header() {
                response.headers_mut().insert(ALLOW, value);
            }

            Box::pin(async { Ok(response) })
        }
    }
}
