use std::time::Duration;
use tokio::time;

use super::{BoxFuture, Middleware, Next};
use crate::{Error, Request};

pub struct Timeout<T> {
    duration: Duration,
    middleware: T,
}

/// Create a new `Timeout` middleware with the specified duration.
pub fn timeout<M, T>(duration: Duration, middleware: M) -> Timeout<M>
where
    M: Middleware<T>,
{
    Timeout {
        duration,
        middleware,
    }
}

impl<M, T> Middleware<T> for Timeout<M>
where
    M: Middleware<T>,
{
    fn call(&self, request: Request<T>, next: Next<T>) -> BoxFuture {
        let future = time::timeout(self.duration, self.middleware.call(request, next));

        Box::pin(async {
            future.await.unwrap_or_else(|_| {
                let message = "response timed out".to_owned();
                Err(Error::gateway_timeout(message.into()))
            })
        })
    }
}
