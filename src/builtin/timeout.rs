use std::time::Duration;
use tokio::time;

use crate::{BoxFuture, Middleware, Next, Request};

pub struct Timeout {
    duration: Duration,
}

/// Create a new `Timeout` middleware with the specified duration.
pub fn timeout(duration: Duration) -> Timeout {
    Timeout { duration }
}

impl<T> Middleware<T> for Timeout {
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        let future = time::timeout(self.duration, next.call(request));
        Box::pin(async { future.await.unwrap_or_else(|_| Err(crate::error!(504))) })
    }
}
