use smallvec::SmallVec;

use super::{ArcMiddleware, BoxFuture};
use crate::{Error, Request, Response, Result};

pub struct Next<State = ()> {
    stack: SmallVec<[ArcMiddleware<State>; 1]>,
}

impl<State> Next<State> {
    pub(crate) fn new(stack: SmallVec<[ArcMiddleware<State>; 1]>) -> Self {
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
