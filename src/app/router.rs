use std::sync::Arc;

use crate::middleware::Middleware;

pub type Router<State> = via_router::Router<Arc<dyn Middleware<State>>>;

/// A mutable route entry bound to a path segment pattern.
///
/// Route definitions are composable and inherit middleware from their
/// ancestors. The order in which routes are defined and the middleware they
/// contain describe the order of operations that occur when a user visits a
/// given route.
///
/// A well-structured application strategically defines middleware in a way
/// where shared behavior is expressed by a common path segment prefix and
/// a linear execution sequence.
///
/// # Example
///
/// ```no_run
/// use std::time::Duration;
/// use via::{App, Request, Next, rescue, timeout};
///
/// let mut app = App::new(());
/// let mut api = app.route("/api");
///
/// // If an error occurs on a descendant of /api, respond with json.
/// // Siblings of /api must define their own error handling logic.
/// api.middleware(rescue(|sanitizer| {
///     sanitizer.respond_with_json();
/// }));
///
/// // If a descendant of /api takes more 10 seconds to respond, return an
/// // error. A practical solution to the common engineering task:
/// //
/// // Don't wait indefinitely for a database connection.
/// //
/// // Since we defined this middleware after the rescue middleware, timeout
/// // errors will generate the following response:
/// //
/// // {
/// //   "status": 503,
/// //   "errors": [{ "message": "Service Unavailable" }]
/// // }
/// api.middleware(timeout(Duration::from_secs(10)));
///
/// // Define our /users resource as a child of /api. Anytime a user visits an
/// // /api/users/* route, the middleware functions that we attached to the
/// // /api namespace are called unconditionally before a response is generated
/// // from any of the route handlers passed to `.respond()`.
/// api.route("/users").scope(|resource| {
///     let todo = async |_: Request, _: Next| todo!();
///
///     // GET /api/users ~> list users
///     resource.respond(via::get(todo));
///
///     // GET /api/users/:id ~> find user with id = :id
///     resource.route("/:id").respond(via::get(todo));
/// });
/// ```
///
pub struct Route<'a, State> {
    pub(super) inner: via_router::RouteMut<'a, Arc<dyn Middleware<State>>>,
}

impl<State> Route<'_, State> {
    /// Append the provided middleware to the route's call stack.
    ///
    /// Middleware that is attached to a route runs unconditionally when a user
    /// visits an ancestor of the route to which it belongs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use via::{App, Request, Next, cookies, raise};
    ///
    /// let mut app = App::new(());
    ///
    /// // Provides application-wide support for request and response cookies.
    /// app.middleware(cookies::unencoded());
    ///
    /// // Requests made to /admin or any of it's descendants must have an
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
    /// Business logic that is defined in middleware that is passed to respond
    /// is only called when the request path matches the route exactly.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use via::{App, Request, Next};
    ///
    /// // Used as a placeholder for middleware that is not yet implemented.
    /// async fn todo(_: Request, _: Next) -> via::Result {
    ///     Err(raise!(500, message = "Todo"))
    /// }
    ///
    /// let mut app = App::new(());
    /// let mut users = app.route("/users");
    ///
    /// // Called before any subsequent middlewares in this scope.
    /// users.middleware(todo);
    ///
    /// // Called only when the request path is /users/<id>.
    /// users.route("/:id").respond(via::get(todo));
    ///
    /// // Called only when the request path is /users.
    /// users.respond(via::get(todo));
    /// ```
    ///
    pub fn respond<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.inner.respond(Arc::new(middleware));
    }

    /// Returns a new route with the provided suffix applied to self.
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
    /// ```no_run
    /// use std::sync::Arc;
    /// use std::time::Instant;
    /// use via::{App, Middleware, Next, Request};
    ///
    /// /// Partial application of a named middleware timer.
    /// struct Timer(String);
    ///
    /// let mut app = App::new(());
    ///
    /// app.route("/users").scope(|resource| {
    ///     let timer = Timer::new("users");
    ///     let todo = async |_: Request, _: Next| todo!();
    ///
    ///     resource.respond(timer.apply(via::get(todo)));
    ///     resource.route("/:id").respond(timer.apply(via::get(todo)));
    /// });
    ///
    /// impl Timer {
    ///     fn new(name: &str) -> Self {
    ///         Self(name.to_owned())
    ///     }
    ///
    ///     fn apply<State, T>(&self, middleware: T) -> impl Middleware<State> + 'static
    ///     where
    ///         T: Middleware<State> + 'static,
    ///         State: Send + Sync,
    ///     {
    ///         let name: Arc<str> = self.0.clone().into();
    ///
    ///         move |request: Request<State>, next: Next<State>| {
    ///             let name = Arc::clone(&name);
    ///
    ///             let started_at = Instant::now();
    ///             let future = middleware.call(request, next);
    ///
    ///             async move {
    ///                 let response = future.await?;
    ///                 let elapsed = started_at.duration_since(Instant::now());
    ///
    ///                 println!(
    ///                     "timer(name = {}): took {} nanoseconds",
    ///                     name, elapsed.as_nanos()
    ///                 );
    ///
    ///                 Ok(response)
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    pub fn scope(mut self, scope: impl FnOnce(&mut Self)) {
        scope(&mut self);
    }
}
