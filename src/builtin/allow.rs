use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::middleware::{BoxFuture, Middleware};
use crate::{Next, Request, Response};

/// Middleware for routing based on the HTTP method of the request.
///
/// When a route has different behavior depending on the HTTP method, grouping
/// the various handlers into an [`Allow`] middleware can prevent the middleware
/// stack from reallocating during request routing and reduce per-method atomic
/// operations into a single operation.
///
/// If the request method is not supported, [`Allow`] automatically returns a
/// `405 Method Not Allowed` response and sets the `Allow` header with the list
/// of permitted methods. You can override this behavior and instead call the
/// next middleware in the stack with [`Allow::or_next`].
///
pub struct Allow<State> {
    allowed: Vec<(Method, Box<dyn Middleware<State>>)>,
    or_next: bool,
}

/// Forward `CONNECT` requests to the provided middleware.
///
pub fn connect<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::CONNECT, middleware)
}

/// Forward `DELETE` requests to the provided middleware.
///
pub fn delete<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::DELETE, middleware)
}

/// Forward `GET` requests to the provided middleware.
///
pub fn get<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::GET, middleware)
}

/// Forward `HEAD` requests to the provided middleware.
///
pub fn head<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::HEAD, middleware)
}

/// Forward `OPTIONS` requests to the provided middleware.
///
pub fn options<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::OPTIONS, middleware)
}

/// Forward `PATCH` requests to the provided middleware.
///
pub fn patch<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::PATCH, middleware)
}

/// Forward `POST` requests to the provided middleware.
///
pub fn post<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::POST, middleware)
}

/// Forward `PUT` requests to the provided middleware.
///
pub fn put<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::PUT, middleware)
}

/// Forward `TRACE` requests to the provided middleware.
///
pub fn trace<State, T>(middleware: T) -> Allow<State>
where
    T: Middleware<State> + 'static,
{
    Allow::new(Method::TRACE, middleware)
}

impl<State> Allow<State> {
    /// Combine the HTTP methods that allowed by `self` with the provided
    /// argument.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Request, Response, Next};
    /// #
    /// # macro_rules! define {
    /// #     $($($ident:ident),*) => { $(let $ident = responder;)* }
    /// # }
    /// #
    /// # async fn responder(_: Request, _: Next) -> via::Result {
    /// #     Response::build().finish()
    /// # }
    /// #
    /// # define!(index, create, show, update, destroy);
    /// #
    /// # let mut app = App::new(());
    /// #
    /// // A typical REST endpoint.
    /// app.at("/users").scope(|users| {
    ///     users.respond(via::get(index).and(via::post(create)));
    ///     users.at("/:id").respond(
    ///         via::get(show)
    ///             .and(via::patch(update))
    ///             .and(via::delete(destroy)),
    ///     );
    /// });
    /// ```
    ///
    pub fn and(mut self, also: Allow<State>) -> Self {
        self.allowed.extend(also.allowed);
        self.or_next = self.or_next || also.or_next;
        self
    }

    /// Forward the request to the next middleware in the stack if no allowed
    /// method is matched.
    ///
    /// If this is the last middleware in the stack, a `404 Not Found` response
    /// is returned.
    ///
    /// This lets you override the default `405 Method Not Allowed` response.
    ///
    pub fn or_next(mut self) -> Self {
        self.or_next = true;
        self
    }
}

impl<State> Allow<State> {
    fn new(method: Method, middleware: impl Middleware<State> + 'static) -> Self {
        Self {
            allowed: vec![(method, Box::new(middleware))],
            or_next: false,
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
            let mut response = Response::from(crate::error!(405).as_json());

            if let Some(value) = self.allow_header() {
                response.headers_mut().insert(ALLOW, value);
            }

            Box::pin(async { Ok(response) })
        }
    }
}
