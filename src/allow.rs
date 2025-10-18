use http::header::ALLOW;
use http::{Method, StatusCode};

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
    #[allow(clippy::type_complexity)]
    or_else: Option<Box<dyn Fn(&Method, String) -> crate::Result + Send + Sync>>,
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

    /// Call the provided function to generate a response if the request method
    /// is not supported.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Next, Request, Response};
    /// #
    /// # async fn greet(request: Request, _: Next) -> via::Result {
    /// #   let name = request.param("name").into_result()?;
    /// #   Response::build().text(format!("Hello, {}!", name))
    /// # }
    /// #
    /// # fn main() {
    /// # let mut app = App::new(());
    /// app.route("/hello/:name").respond(
    ///     // curl -XPOST http://localhost:8080/hello/world
    ///     // => Method Not Allowed: POST
    ///     via::get(greet).or_else(|method, allowed| {
    ///         Response::build()
    ///             .status(405)
    ///             .header("Allow", allowed)
    ///             .text(format!("Method Not Allowed: {}", method))
    ///     })
    /// );
    /// # }
    /// ```
    ///
    pub fn or_else<F>(mut self, or_else: F) -> Self
    where
        F: Fn(&Method, String) -> crate::Result + Send + Sync + 'static,
    {
        self.or_else = Some(Box::new(or_else));
        self
    }

    /// Return a `405 Method Not Allowed` response if the request method is not
    /// supported.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Next, Request, Response};
    /// #
    /// # async fn greet(request: Request, _: Next) -> via::Result {
    /// #    let name = request.param("name").into_result()?;
    /// #    Response::build().text(format!("Hello, {}!", name))
    /// # }
    /// #
    /// # fn main() {
    /// # let mut app = App::new(());
    /// app.route("/hello/:name").respond(
    ///     // curl -XPOST http://localhost:8080/hello/world
    ///     // => Method Not Allowed: POST
    ///     via::get(greet).or_not_allowed()
    /// );
    /// # }
    /// ```
    ///
    pub fn or_not_allowed(self) -> Self {
        self.or_else(|method, allowed| {
            Response::build()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(ALLOW, allowed)
                .text(format!("Method not allowed: {}.", method))
        })
    }
}

impl<State> Allow<State> {
    fn new<T>(method: Method, middleware: T) -> Self
    where
        T: Middleware<State> + 'static,
    {
        Self {
            allowed: vec![(method, Box::new(middleware))],
            or_else: None,
        }
    }

    fn allow_header(&self) -> String {
        self.allowed
            .iter()
            .map(|(method, _)| method.as_str())
            .fold(String::new(), |allowed, method| allowed + ", " + method)
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
        let method = request.method();

        if let Some(middleware) = self.respond_to(method) {
            middleware.call(request, next)
        } else if let Some(or_else) = self.or_else.as_deref() {
            let result = or_else(method, self.allow_header());
            Box::pin(async { result })
        } else {
            next.call(request)
        }
    }
}
