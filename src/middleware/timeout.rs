use http::StatusCode;
use std::time::Duration;
use tokio::time;

use super::{BoxFuture, Middleware, Next};
use crate::{Error, Request, Response, Result};

/// A type alias for the default `or_else` function.
type RespondWithTimeout<State> = fn(&State) -> Result<Response, Error>;

/// Middleware that calls a fallback function if downstream middleware do not
/// respond within a specified duration.
pub struct Timeout<F> {
    duration: Duration,
    or_else: F,
}

/// Create a new `Timeout` middleware with the specified duration.
pub fn timeout<State>(duration: Duration) -> Timeout<RespondWithTimeout<State>> {
    Timeout::new(duration)
}

/// The default function to call if downstream middleware do not respond within
/// the specified duration.
fn respond_with_timeout<State>(_: &State) -> Result<Response, Error> {
    let mut message = String::with_capacity(65);
    let status = StatusCode::GATEWAY_TIMEOUT;

    message.push_str("The server is taking too long to respond. ");
    message.push_str("Please try again later.");

    Ok(Error::new_with_status(message, status).into_response())
}

impl<State> Timeout<RespondWithTimeout<State>> {
    /// Create a new `Timeout` middleware with the specified duration.
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            or_else: respond_with_timeout,
        }
    }
}

impl<F> Timeout<F> {
    /// Call the specified function instead of responding with a 504 Gateway
    /// Timeout error if the downstream middleware do not respond within
    /// `self.duration`.
    pub fn or_else<State, O>(self, f: O) -> Timeout<O>
    where
        O: Fn(&State) -> Result<Response, Error> + Copy + Send + Sync + 'static,
    {
        Timeout {
            duration: self.duration,
            or_else: f,
        }
    }
}

impl<State, F> Middleware<State> for Timeout<F>
where
    F: Fn(&State) -> Result<Response, Error> + Copy + Send + Sync + 'static,
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture<Result<Response>> {
        let duration = self.duration;
        let or_else = self.or_else;
        let state = request.state().clone();

        Box::pin(async move {
            match time::timeout(duration, next.call(request)).await {
                Ok(result) => result,
                Err(_) => or_else(&state),
            }
        })
    }
}
