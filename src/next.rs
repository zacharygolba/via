use std::collections::VecDeque;
use std::sync::Arc;

use crate::middleware::{BoxFuture, Middleware};
use crate::request::Request;

/// The next middleware in the logical call stack of a request.
///
pub struct Next<State = ()> {
    deque: VecDeque<Arc<dyn Middleware<State>>>,
}

impl<State> Next<State> {
    #[inline]
    pub(crate) fn new(deque: VecDeque<Arc<dyn Middleware<State>>>) -> Self {
        Self { deque }
    }

    /// Calls the next middleware in the logical call stack of the request.
    ///
    /// # Example
    ///
    /// ```
    /// use via::{Request, Next};
    ///
    /// async fn logger(request: Request, next: Next) -> via::Result {
    ///     let head = request.envelope();
    ///
    ///     println!("{} -> {}", head.method(), head.uri().path());
    ///     next.call(request).await.inspect(|response| {
    ///         println!("<- {}", response.status());
    ///     })
    /// }
    /// ```
    pub fn call(mut self, request: Request<State>) -> BoxFuture {
        match self.deque.pop_front() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async { crate::raise!(404) }),
        }
    }
}
