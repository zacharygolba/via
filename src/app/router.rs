use std::sync::Arc;

use crate::middleware::Middleware;

pub type Router<State> = via_router::Router<Arc<dyn Middleware<State>>>;

/// An entry in the route tree associated with a path segment pattern.
///
/// Route definitions are composable and inherit middleware from their
/// ancestors. The order in which routes and their middleware are defined
/// determines the sequence of operations that occur when a user visits a given
/// route.
///
/// A well-structured application strategically defines middleware so that
/// shared behavior is expressed by a common path segment prefix and ordered
/// to reflect its execution sequence.
///
/// # Example
///
/// ```no_run
/// use std::process::ExitCode;
/// use via::error::{Error, Rescue};
/// use via::{App, Next, Request, Server, Timeout};
///
/// #[tokio::main]
/// async fn main() -> Result<ExitCode, Error> {
///     let mut app = App::new(());
///     let mut api = app.route("/api");
///
///     // If an error occurs on a descendant of /api, respond with json.
///     // Siblings of /api must define their own error handling logic.
///     api.middleware(Rescue::with(|sanitizer| sanitizer.use_json()));
///
///     // If a descendant of /api takes more 10 seconds to respond, return an
///     // error. A practical solution to the common engineering task: Don't
///     // wait indefinitely for a database connection.
///     //
///     // Since we defined our timeout middleware after the rescue middleware,
///     // timeout errors will generate the following response:
///     //
///     // {
///     //   "status": 503,
///     //   "errors": [{ "message": "Service Unavailable" }]
///     // }
///     api.middleware(Timeout::from_secs(10).or_service_unavailable());
///
///     // Define a /users resource as a child of /api so the rescue and timeout
///     // middleware run before any of the middleware or responders defined in
///     // the /users resource.
///     api.route("/users").scope(|resource| {
///         let todo = async |_: Request, _: Next| todo!();
///
///         // GET /api/users ~> list users
///         resource.respond(via::get(todo));
///
///         // GET /api/users/:id ~> find user with id = :id
///         resource.route("/:id").respond(via::get(todo));
///     });
///
///     // Start serving our application from http://localhost:8080/.
///     Server::new(app).listen(("127.0.0.1", 8080)).await
/// }
///
/// ```
///
pub struct Route<'a, State> {
    pub(super) inner: via_router::RouteMut<'a, Arc<dyn Middleware<State>>>,
}

impl<State> Route<'_, State> {
    /// Appends the provided middleware to the route's call stack.
    ///
    /// Middleware attached to a route runs anytime the route’s path is a
    /// prefix of the request path.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Request, Next, raise};
    /// # let mut app = App::new(());
    /// #
    /// // Provides application-wide support for request and response cookies.
    /// app.middleware(via::cookies::unencoded());
    ///
    /// // Requests made to /admin or any of its descendants must have an
    /// // is_admin cookie present on the request.
    /// app.route("/admin").middleware(async |request: Request, next: Next| {
    ///     // We suggest using signed cookies to prevent tampering.
    ///     // See the cookies example in our git repo for more information.
    ///     if request.cookies().get("is_admin").is_some() {
    ///         next.call(request).await
    ///     } else {
    ///         Err(raise!(401))
    ///     }
    /// });
    /// ```
    ///
    pub fn middleware<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.inner.middleware(Arc::new(middleware));
    }

    /// Defines how the route should respond when it is visited.
    ///
    /// Middleware passed to `respond` runs only when the request path matches
    /// the route exactly.
    ///
    /// # Example
    ///
    /// ```
    /// # use via::{App, Request, Next};
    /// # let mut app = App::new(());
    /// let mut users = app.route("/users");
    ///
    /// // Called only when the request path is /users.
    /// users.respond(via::get(async |_, _| todo!()));
    ///
    /// // Called only when the request path matches /users/:id.
    /// users.route("/:id").respond(via::get(async |_, _| todo!()));
    /// ```
    ///
    pub fn respond<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.inner.respond(Arc::new(middleware));
    }

    /// Returns a new child route by appending the provided path to the current
    /// route.
    ///
    /// The path argument can contain multiple segments. The returned route
    /// always represents the final segment of that path.
    ///
    /// # Example
    ///
    /// ```
    /// # let mut app = via::App::new(());
    /// // The following routes reference the router entry at /hello/:name.
    /// app.route("/hello/:name");
    /// app.route("/hello").route("/:name");
    /// ```
    ///
    /// # Dynamic Segments
    ///
    /// Routes can include *dynamic* segments that capture portions of the
    /// request path as parameters. These parameters are made available to
    /// middleware at runtime.
    ///
    /// - `:dynamic` — Matches a single path segment. `/users/:id` matches
    ///   `/users/12345` and captures `"12345"` as `id`.
    ///
    /// - `*splat` — Matches zero or more remaining path segments.
    ///   `/static/*asset` matches `/static/logo.png` or `/static/css/main.css`
    ///   and captures the remainder of the path starting from the splat
    ///   pattern as `asset`. `logo.png` and `css/main.css`.
    ///
    /// Dynamic segments match any path segment, so define them after all
    /// static sibling routes to ensure intended routing behavior.
    ///
    /// Consider the following sequence of route definitions. We define
    /// `/articles/trending` before `/articles/:id` to ensure that a request to
    /// `/articles/trending` is routed to `articles::trending` rather than
    /// capturing `"trending"` as `id` and invoking `articles::show`.
    ///
    /// ```
    /// # let mut app = via::App::new(());
    /// #
    /// app.route("/articles").scope(|resource| {
    ///     // list articles
    ///     resource.respond(via::get(articles::index));
    ///     // list trending articles
    ///     resource.route("/trending").respond(via::get(articles::trending));
    ///     // find article with id = :id
    ///     resource.route("/:id").respond(via::get(articles::show));
    /// });
    /// #
    /// # mod articles {
    /// #     use via::{Next, Request};
    /// #     pub async fn trending(_: Request, _: Next) -> via::Result { todo!() }
    /// #     pub async fn index(_: Request, _: Next) -> via::Result { todo!() }
    /// #     pub async fn show(_: Request, _: Next) -> via::Result { todo!() }
    /// # }
    /// ```
    ///
    pub fn route(&mut self, path: &'static str) -> Route<'_, State> {
        Route {
            inner: self.inner.route(path),
        }
    }

    /// Consumes self by calling the provided closure with a mutable reference
    /// to self.
    ///
    /// # Example
    ///
    /// ```
    /// # let mut app = via::App::new(());
    /// #
    /// # mod users {
    /// #     use via::{Request, Next};
    /// #     pub async fn index(_: Request, _: Next) -> via::Result { todo!() }
    /// #     pub async fn show(_: Request, _: Next) -> via::Result { todo!() }
    /// # }
    /// #
    /// app.route("/users").scope(|resource| {
    ///     // Imports within a scope can be specific. No need to worry about
    ///     // `index` being shadowed by another route definition.
    ///     use users::index;
    ///
    ///     // Scopes can also be used to define intermittent variables to
    ///     // compose middleware without polluting the top-level scope of the
    ///     // main fn.
    ///     let show = via::get(users::show);
    ///
    ///     // List users.
    ///     resource.respond(via::get(index));
    ///
    ///     // Find user with id = :id.
    ///     resource.route("/:id").respond(show);
    /// });
    /// ```
    ///
    pub fn scope(mut self, scope: impl FnOnce(&mut Self)) {
        scope(&mut self);
    }
}
