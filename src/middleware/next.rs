use super::ArcMiddleware;
use crate::{BoxFuture, Error, Request, Response, Result};

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
            Box::pin(async { Response::text("Not Found").status(404).finish() })
        }
    }
}
