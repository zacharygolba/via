use std::collections::VecDeque;

use super::ArcMiddleware;
use crate::{BoxFuture, Request, Response, Result};

pub struct Next<State = ()> {
    #[allow(clippy::box_collection)]
    stack: Box<VecDeque<ArcMiddleware<State>>>,
}

impl<State> Next<State>
where
    State: Send + Sync + 'static,
{
    pub(crate) fn new(stack: VecDeque<ArcMiddleware<State>>) -> Self {
        Self {
            stack: Box::new(stack),
        }
    }

    pub fn call(mut self, request: Request<State>) -> BoxFuture<Result<Response>> {
        if let Some(middleware) = self.stack.pop_front() {
            middleware.call(request, self)
        } else {
            Box::pin(async { Response::text("Not Found").status(404).end() })
        }
    }
}
