use std::time::Duration;
use tokio::time;

use super::{BoxFuture, Middleware, Next};
use crate::{Error, Request};

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

        Box::pin(async {
            future.await.unwrap_or_else(|_| {
                let message = "response timed out".to_owned();
                Err(Error::gateway_timeout(message.into()))
            })
        })
    }
}
