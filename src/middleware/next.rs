use std::collections::VecDeque;

use super::DynMiddleware;
use crate::{Request, Response, Result};

pub struct Next {
    stack: VecDeque<DynMiddleware>,
}

impl Next {
    pub(crate) fn new(stack: VecDeque<DynMiddleware>) -> Self {
        Next { stack }
    }

    pub async fn call(mut self, request: Request) -> Result<Response> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.as_ref().call(request, self).await
        } else {
            Response::text("Not Found").status(404).end()
        }
    }
}
