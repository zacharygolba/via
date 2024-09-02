use http::StatusCode;
use std::time::Duration;
use tokio::time;

use super::{BoxFuture, Middleware, Next};
use crate::{Error, Request, Response, Result};

/// Middleware that calls a fallback function if downstream middleware do not
/// respond within a specified duration.
pub struct Timeout<F> {
    duration: Duration,
    or_else: F,
}

/// Create a new `Timeout` middleware with the specified duration.
pub fn timeout(duration: Duration) -> Timeout<fn() -> Result<Response, Error>> {
    Timeout {
        duration,
        or_else: respond_with_timeout,
    }
}

/// The default function to call if downstream middleware do not respond within
/// the specified duration.
fn respond_with_timeout() -> Result<Response, Error> {
    let mut message = String::with_capacity(65);
    let status = StatusCode::GATEWAY_TIMEOUT;

    message.push_str("The server is taking too long to respond. ");
    message.push_str("Please try again later.");

    Ok(Error::with_status(message, status).into_response())
}

impl<F> Timeout<F> {
    /// Call the specified function instead of responding with a 504 Gateway
    /// Timeout error if the downstream middleware do not respond within
    /// `self.duration`.
    pub fn or_else<O>(self, f: O) -> Timeout<O>
    where
        O: Fn() -> Result<Response, Error> + Copy + Send + Sync + 'static,
    {
        Timeout {
            duration: self.duration,
            or_else: f,
        }
    }
}

impl<State, F> Middleware<State> for Timeout<F>
where
    F: Fn() -> Result<Response, Error> + Copy + Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let or_else = self.or_else;
        let future = time::timeout(self.duration, next.call(request));

        Box::pin(async move { future.await.unwrap_or_else(|_| or_else()) })
    }
}
