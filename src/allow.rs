use http::header::ALLOW;
use http::{HeaderValue, Method};

use crate::middleware::{BoxFuture, Middleware};
use crate::{Next, Request, Response};

/// HTTP method based middleware dispatch.
///
/// When a route’s behavior varies by method, grouping the handlers with
/// [`Allow`] can prevent middleware stack reallocations and collapse
/// per-method atomic operations into a single one.
///
/// # Example
///
/// ```no_run
/// use std::process::ExitCode;
/// use via::{App, BoxError, Server};
///
/// mod users {
///     use via::{Next, Request};
///
///     pub async fn create(_: Request, _: Next) -> via::Result { todo!() }
///     pub async fn destroy(_: Request, _: Next) -> via::Result { todo!() }
///     pub async fn list(_: Request, _: Next) -> via::Result { todo!() }
///     pub async fn show(_: Request, _: Next) -> via::Result { todo!() }
///     pub async fn update(_: Request, _: Next) -> via::Result { todo!() }
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<ExitCode, BoxError> {
///     let mut app = App::new(());
///
///     // HTTP method based dispatch.
///     app.route("/users").scope(|resource| {
///         resource.respond(via::get(users::list).post(users::create));
///         resource.route("/:id").respond(
///             via::get(users::show)
///                 .patch(users::update)
///                 .delete(users::destroy)
///         );
///     });
///
///     Server::new(app).listen(("127.0.0.1", 8080)).await
/// }
///
pub struct Allow<State> {
    allowed: Vec<(Method, Box<dyn Middleware<State>>)>,
    or_next: bool,
}

macro_rules! allow_factory {
    ( $( $vis:vis fn $name:ident($method:ident) ),* $(,)? ) => {
        $(
            #[doc = docs_for!($method)]
            $vis fn $name<State, T>(middleware: T) -> Allow<State>
            where
                T: Middleware<State> + 'static,
            {
                Allow::new(Method::$method, middleware)
            }
        )*
    };
}

macro_rules! docs_for {
    ($method:ident) => {
        concat!(
            "Route `",
            stringify!($method),
            "` requests to the provided middleware."
        )
    };
}

macro_rules! extend_allowed {
    ( $( $vis:vis fn $name:ident($method:ident) ),* $(,)? ) => {
        $(
            #[doc = docs_for!($method)]
            $vis fn $name<T>(mut self, middleware: T) -> Self
            where
                T: Middleware<State> + 'static,
            {
                self.allowed.push((Method::$method, Box::new(middleware)));
                self
            }
        )*
    };
}

allow_factory!(
    pub fn connect(CONNECT),
    pub fn delete(DELETE),
    pub fn get(GET),
    pub fn head(HEAD),
    pub fn options(OPTIONS),
    pub fn patch(PATCH),
    pub fn post(POST),
    pub fn put(PUT),
    pub fn trace(TRACE),
);

impl<State> Allow<State> {
    extend_allowed!(
        pub fn connect(CONNECT),
        pub fn delete(DELETE),
        pub fn get(GET),
        pub fn head(HEAD),
        pub fn options(OPTIONS),
        pub fn patch(PATCH),
        pub fn post(POST),
        pub fn put(PUT),
        pub fn trace(TRACE),
    );

    /// Return a `405 Method Not Allowed` response if the request method is not
    /// supported.
    ///
    pub fn or_method_not_allowed(mut self) -> Self {
        self.or_next = false;
        self
    }
}

impl<State> Allow<State> {
    fn new<T>(method: Method, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
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
