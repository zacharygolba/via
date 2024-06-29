use std::collections::VecDeque;

use super::DynMiddleware;
use crate::{BoxFuture, Request, Response, Result};

pub struct Next {
    stack: VecDeque<DynMiddleware>,
}

impl Next {
    pub(crate) fn new(stack: VecDeque<DynMiddleware>) -> Self {
        Next { stack }
    }

    pub fn call(mut self, request: Request) -> BoxFuture<Result<Response>> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(request, self)
        } else {
            Box::pin(async { Response::text("Not Found").status(404).end() })
        }
    }
}
