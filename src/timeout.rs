use std::time::Duration;
use tokio::time;

use crate::middleware::{BoxFuture, Middleware};
use crate::{Next, Request};

pub struct Timeout {
    duration: Duration,
}

/// Create a new `Timeout` middleware with the specified duration.
pub fn timeout(duration: Duration) -> Timeout {
    Timeout { duration }
}

impl<State> Middleware<State> for Timeout
where
    State: Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let Self { duration } = *self;

        Box::pin(async move {
            match time::timeout(duration, next.call(request)).await {
                Ok(result) => result,
                Err(_) => Err(crate::raise!(504)),
            }
        })
    }
}
