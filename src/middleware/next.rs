use super::middleware::{ArcMiddleware, BoxFuture};
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

pub struct Next<T = ()> {
    stack: Vec<ArcMiddleware<T>>,
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new(stack: Vec<ArcMiddleware<T>>) -> Self {
        Self { stack }
    }

    #[inline]
    pub fn call(mut self, request: Request<T>) -> BoxFuture<Result<Response, Error>> {
        match self.stack.pop() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async {
                let message = "not found".to_owned();
                Err(Error::not_found(message.into()))
            }),
        }
    }
}
