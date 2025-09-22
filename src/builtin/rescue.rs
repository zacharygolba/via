use crate::middleware::{BoxFuture, Middleware};
use crate::{Error, Next, Request};

pub struct Inspect<F> {
    inspect: F,
}

pub struct Map<F> {
    map: F,
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to inspect the error to another error. Think of this as a
/// [`Result::inspect_err`] for middleware.
///
pub fn inspect<F>(inspect: F) -> Inspect<F>
where
    F: Fn(&Error) + Copy + Send + Sync + 'static,
{
    Inspect { inspect }
}

/// A middleware that catches errors that occur downstream and then calls the
/// provided closure to map the error to another error. Think of this as a
/// `Result::map` for middleware.
///
pub fn map<F>(map: F) -> Map<F>
where
    F: Fn(Error) -> Error + Copy + Send + Sync + 'static,
{
    Map { map }
}

impl<State, F> Middleware<State> for Inspect<F>
where
    State: Send + Sync + 'static,
    F: Fn(&Error) + Copy + Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let inspect = self.inspect;

        Box::pin(async move {
            next.call(request).await.or_else(|error| {
                inspect(&error);
                Ok(error.into())
            })
        })
    }
}

impl<State, F> Middleware<State> for Map<F>
where
    State: Send + Sync + 'static,
    F: Fn(Error) -> Error + Copy + Send + Sync + 'static,
{
    fn call(&self, request: Request<State>, next: Next<State>) -> BoxFuture {
        let map = self.map;

        Box::pin(async move {
            let result = next.call(request).await;
            result.or_else(|error| Ok(map(error).into()))
        })
    }
}
