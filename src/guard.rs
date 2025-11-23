use crate::middleware::BoxFuture;
use crate::{Middleware, Next, Request};

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
