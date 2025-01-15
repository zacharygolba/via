use super::middleware::{ArcMiddleware, BoxFuture};
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

pub struct Next<State = ()> {
    stack: Vec<ArcMiddleware<State>>,
}

impl<State> Next<State> {
    pub(crate) fn new(stack: Vec<ArcMiddleware<State>>) -> Self {
        Self { stack }
    }

    #[inline]
    pub fn call(mut self, request: Request<State>) -> BoxFuture<Result<Response, Error>> {
        match self.stack.pop() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async {
                let message = "not found".to_owned();
                Err(Error::not_found(message.into()))
            }),
        }
    }
}
