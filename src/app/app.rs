use super::shared::Shared;
use crate::middleware::Middleware;
use crate::router::{Route, Router};

/// Configure routes and define shared global state.
///
/// # Example
///
/// ```no_run
/// use std::process::ExitCode;
/// use via::{App, Error, Next, Request, Server};
///
/// /// A mock database pool.
/// #[derive(Debug)]
/// struct DatabasePool {
///     url: String,
/// }
///
/// /// Shared global state. Named after our application.
/// struct Unicorn {
///     pool: DatabasePool,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<ExitCode, Error> {
///     // Pass our shared state struct containing a database pool to the App
///     // constructor so it can be used to serve each request.
///     let mut app = App::new(Unicorn {
///         pool: DatabasePool {
///             url: std::env::var("DATABASE_URL")?,
///         },
///     });
///
///     // We can access our database in middleware with `request.state()`.
///     app.uses(async |request: Request<Unicorn>, next: Next<Unicorn>| {
///         // Get a reference to the state argument passed to `App::new`.
///         let state = request.state();
///
///         // Print the debug output of our mock database pool to stdout.
///         println!("{:?}", &state.pool);
///
///         // Delegate to the next middleware to get a response.
///         next.call(request).await
///     });
///
///     // Start serving our application from http://localhost:8080/.
///     Server::new(app).listen(("127.0.0.1", 8080)).await
/// }
/// ```
///
pub struct Via<App> {
    pub(super) state: Shared<App>,
    pub(super) router: Router<App>,
}

/// Create a new app with the provided state argument.
///
/// # Example
///
/// ```
/// # struct DatabasePool { url: String }
/// # struct Unicorn { pool: DatabasePool }
/// #
/// let mut app = via::app(Unicorn {
///     pool: DatabasePool {
///         url: "postgres://unicorn@localhost/unicorn".to_owned(),
///     },
/// });
/// ```
///
pub fn app<App>(app: App) -> Via<App> {
    Via {
        state: Shared::new(app),
        router: Router::new(),
    }
}

impl<State> Via<State> {
    /// Returns a new route as a child of the root path `/`.
    ///
    /// See also the usage example in [`Route::route`].
    ///
    pub fn route(&mut self, path: &'static str) -> Route<'_, State> {
        self.router.route(path)
    }

    /// Append the provided middleware to applications call stack.
    ///
    /// Middleware attached with this method runs for every request.
    ///
    /// See also the usage example in [`Route::uses`].
    ///
    pub fn uses<T>(&mut self, middleware: T)
    where
        T: Middleware<State> + 'static,
    {
        self.route("/").uses(middleware);
    }
}
