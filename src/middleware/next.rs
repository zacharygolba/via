use std::collections::VecDeque;
use std::sync::Arc;

use super::middleware::{FutureResponse, Middleware};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    stack: VecDeque<Arc<dyn Middleware<T>>>,
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            stack: VecDeque::new(),
        }
    }

    #[inline]
    pub(crate) fn stack_mut(&mut self) -> &mut VecDeque<Arc<dyn Middleware<T>>> {
        &mut self.stack
    }

    pub fn call(mut self, request: Request<T>) -> FutureResponse {
        match self.stack.pop_front() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async {
                let message = "not found".to_owned();
                Err(Error::not_found(message.into()))
            }),
        }
    }
}
