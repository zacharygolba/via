use std::collections::VecDeque;

use super::DynMiddleware;
use crate::{BoxFuture, Request, Response, Result};

pub struct Next<State> {
    stack: VecDeque<DynMiddleware<State>>,
}

impl<State> Next<State>
where
    State: Send + Sync + 'static,
{
    pub(crate) fn new(stack: VecDeque<DynMiddleware<State>>) -> Self {
        Next { stack }
    }

    pub fn call(mut self, request: Request<State>) -> BoxFuture<Result<Response>> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(request, self)
        } else {
            Box::pin(async { Response::text("Not Found").status(404).end() })
        }
    }
}
