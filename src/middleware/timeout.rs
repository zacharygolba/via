use std::time::Duration;
use tokio::time;

use super::{Middleware, Next};
use crate::{Error, Request};

/// Create a new `Timeout` middleware with the specified duration.
pub fn timeout<T>(duration: Duration) -> impl Middleware<T> {
    move |request: Request<T>, next: Next<T>| {
        let future = time::timeout(duration, next.call(request));

        async {
            future.await.unwrap_or_else(|_| {
                let message = "response timed out".to_owned();
                Err(Error::gateway_timeout(message.into()))
            })
        }
    }
}
