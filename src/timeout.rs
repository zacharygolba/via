use http::StatusCode;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::middleware::{BoxFuture, Middleware};
use crate::{Error, Next, Request};

/// Enforce that downstream middleware respond within a specified duration.
///
/// If the timeout expires before a response is generated, an error is returned.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use tokio::time::sleep;
/// use via::{App, Response, Timeout};
///
/// let mut app = App::new(());
///
/// app.middleware(Timeout::new(Duration::from_secs(10)));
/// app.route("/").respond(via::get(async |_, _| {
///     sleep(Duration::from_secs(11)).await;
///     Response::build().text("Hello, world!")
/// }));
/// ```
///
pub struct Timeout {
    duration: Duration,
    or_else: OrElse,
}

enum OrElse {
    Fallback(Arc<dyn Fn() -> crate::Result + Send + Sync>),
    Status(StatusCode),
}

impl Clone for OrElse {
    fn clone(&self) -> Self {
        match *self {
            Self::Fallback(ref f) => Self::Fallback(Arc::clone(f)),
            Self::Status(status) => Self::Status(status),
        }
    }
}

impl Timeout {
    /// Create a `Timeout` middleware from the duration provided.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::Duration;
    /// use via::{App, Timeout};
    ///
    /// let mut app = App::new(());
    /// app.middleware(Timeout::new(Duration::from_secs(10)));
    /// ```
    ///
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            or_else: OrElse::Status(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    /// Create a `Timeout` middleware with the provided duration in seconds.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{App, Timeout};
    ///
    /// let mut app = App::new(());
    /// app.middleware(Timeout::from_secs(10));
    /// ```
    ///
    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }

    /// If the timeout expires, call the provided closure to generate a
    /// response.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{App, Response, Timeout};
    ///
    /// let mut app = App::new(());
    /// let mut api = app.route("/api");
    ///
    /// api.middleware(Timeout::from_secs(10).or_else(|| {
    ///     Response::build()
    ///         .status(503)
    ///         .header("Retry-After", "30")
    ///         .text("Request timed out. Please try again later.")
    /// }));
    /// ```
    ///
    pub fn or_else<F>(mut self, f: F) -> Self
    where
        F: Fn() -> crate::Result + Send + Sync + 'static,
    {
        self.or_else = OrElse::Fallback(Arc::new(f));
        self
    }

    /// If the timeout expires, respond with a `504 Gateway Timeout` status
    /// code.
    ///
    /// A `504` status code typically indicates that an upstream dependency is
    /// unresponsive. For example, failing to connect to a database.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{App, Timeout};
    ///
    /// let mut app = App::new(());
    /// let mut api = app.route("/api");
    ///
    /// api.middleware(Timeout::from_secs(10).or_gateway_timeout());
    /// ```
    ///
    pub fn or_gateway_timeout(self) -> Self {
        self.with_status(StatusCode::GATEWAY_TIMEOUT)
    }

    /// If the timeout expires, respond with a `503 Service Unavailable` status
    /// code.
    ///
    /// A `503` status code typically indicates a backend dependency failure.
    /// For example, failing to connect to a database.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{App, Timeout};
    ///
    /// let mut app = App::new(());
    /// let mut api = app.route("/api");
    ///
    /// api.middleware(Timeout::from_secs(10).or_service_unavailable());
    /// ```
    ///
    pub fn or_service_unavailable(self) -> Self {
        self.with_status(StatusCode::SERVICE_UNAVAILABLE)
    }
}

impl Timeout {
    fn with_status(mut self, status: StatusCode) -> Self {
        self.or_else = OrElse::Status(status);
        self
    }
}

impl<State> Middleware<State> for Timeout
where
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let duration = self.duration;
        let or_else = self.or_else.clone();

        Box::pin(async move {
            if let Ok(result) = time::timeout(duration, next.call(request)).await {
                return result;
            }

            match or_else {
                OrElse::Fallback(f) => f(),
                OrElse::Status(status) => {
                    let message = status.canonical_reason().unwrap_or_default();
                    Err(Error::new(status, message))
                }
            }
        })
    }
}
