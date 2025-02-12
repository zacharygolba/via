use std::sync::Arc;

use super::middleware::{FutureResponse, Middleware};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    stack: Vec<Arc<dyn Middleware<T>>>,
}

impl<T> Next<T> {
    pub(crate) fn new(stack: Vec<Arc<dyn Middleware<T>>>) -> Self {
        Self { stack }
    }

    pub fn call(mut self, request: Request<T>) -> FutureResponse {
        match self.stack.pop() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async {
                let message = "not found".to_owned();
                Err(Error::not_found(message.into()))
            }),
        }
    }
}
