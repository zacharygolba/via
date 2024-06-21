use std::collections::VecDeque;

use super::DynMiddleware;
use crate::{Context, Response, Result};

pub struct Next {
    stack: VecDeque<DynMiddleware>,
}

impl Next {
    pub(crate) fn new(stack: VecDeque<DynMiddleware>) -> Self {
        Next { stack }
    }

    pub async fn call(mut self, context: Context) -> Result<Response> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.as_ref().call(context, self).await
        } else {
            Response::text("Not Found").status(404).end()
        }
    }
}
