use crate::middleware::BoxFuture;
use crate::{Middleware, Next, Request};

/// Enforce that downstream middleware respond within a specified duration.
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
/// app.uses(Timeout::new(Duration::from_secs(10)));
/// app.route("/").to(via::get(async |_, _| {
///     sleep(Duration::from_secs(11)).await;
///     Response::build().text("Hello, world!")
/// }));
/// ```
///
pub struct Guard<F> {
    check: F,
}

impl<F> Guard<F> {
    pub fn new(check: F) -> Self {
        Self { check }
    }
}

impl<State, R, F> Middleware<State> for Guard<F>
where
    State: Send + Sync + 'static,
    F: Fn(&Request<State>) -> crate::Result<R> + Send + Sync,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        if let Err(error) = (self.check)(&request) {
            Box::pin(async { Err(error) })
        } else {
            next.call(request)
        }
    }
}
