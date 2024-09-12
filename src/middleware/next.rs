use super::ArcMiddleware;
use crate::{BoxFuture, Error, Request, Response, Result};

pub struct Next<State = ()> {
    #[allow(clippy::box_collection)]
    stack: Box<Vec<ArcMiddleware<State>>>,
}

impl<State> Next<State> {
    #[allow(clippy::box_collection)]
    pub(crate) fn new(stack: Box<Vec<ArcMiddleware<State>>>) -> Self {
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
