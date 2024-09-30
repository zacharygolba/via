use super::{ArcMiddleware, BoxFuture};
use crate::{Error, Request, Response, Result};

pub struct Next<State = ()> {
    stack: Vec<ArcMiddleware<State>>,
}

impl<State> Next<State> {
    pub(crate) fn new(stack: Vec<ArcMiddleware<State>>) -> Self {
        Self { stack }
    }

    pub fn call(mut self, request: Request<State>) -> BoxFuture<Result<Response, Error>> {
        if let Some(middleware) = self.stack.pop() {
            middleware.call(request, self)
        } else {
            Box::pin(async { Ok(Response::not_found()) })
        }
    }
}
