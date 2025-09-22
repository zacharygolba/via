use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::middleware::{BoxFuture, Middleware};
use crate::{Next, Request, Response};

pub struct Allow<State> {
    allowed: String,
    methods: Vec<(Method, Box<dyn Middleware<State>>)>,
    or_else: Option<Box<dyn Middleware<State>>>,
}

pub fn delete<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::DELETE, middleware)
}

pub fn get<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::GET, middleware)
}

pub fn head<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::HEAD, middleware)
}

pub fn options<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::OPTIONS, middleware)
}

pub fn patch<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::PATCH, middleware)
}

pub fn post<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::POST, middleware)
}

pub fn put<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::PUT, middleware)
}

pub fn trace<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::TRACE, middleware)
}

impl<State> Allow<State> {
    pub fn and(mut self, other: Allow<State>) -> Self {
        let allowed = &mut self.allowed;

        for (method, _) in &other.methods {
            allowed.push_str(", ");
            allowed.push_str(method.as_str());
        }

        self.methods.extend(other.methods);
        self
    }

    pub fn or_else(mut self, or_else: impl Middleware<State> + 'static) -> Self {
        self.or_else = Some(Box::new(or_else));
        self
    }

    pub fn or_next(self) -> Self {
        self.or_else(|request, next: Next<State>| next.call(request))
    }
}

impl<State> Allow<State> {
    fn new(method: Method, middleware: impl Middleware<State> + 'static) -> Self {
        Self {
            allowed: method.as_str().to_owned(),
            methods: vec![(method, Box::new(middleware))],
            or_else: None,
        }
    }

    fn allow(&self, method: &Method) -> Option<&dyn Middleware<State>> {
        self.methods
            .iter()
            .find_map(|(allow, m)| (method == allow).then_some(m.as_ref()))
            .or(self.or_else.as_deref())
    }

    fn deny(&self, method: &Method) -> BoxFuture {
        let error = crate::error!(405, "request method \"{}\" is not supported", method);
        let mut response = Response::from(error.as_json());

        if let Ok(header) = HeaderValue::from_str(&self.allowed) {
            response.headers_mut().insert(ALLOW, header);
        }

        Box::pin(async { Ok(response) })
    }
}

impl<State> Middleware<State> for Allow<State> {
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let method = request.method();

        if let Some(middleware) = self.allow(method) {
            middleware.call(request, next)
        } else {
            self.deny(method)
        }
    }
}
