use std::collections::VecDeque;
use std::sync::Arc;

use super::middleware::{FutureResponse, Middleware};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    deque: VecDeque<Arc<dyn Middleware<T>>>,
}

impl<T> Next<T> {
    pub fn call(mut self, request: Request<T>) -> FutureResponse {
        match self.deque.pop_front() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async {
                let message = "not found".to_owned();
                Err(Error::not_found(message.into()))
            }),
        }
    }
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new(stack: VecDeque<Arc<dyn Middleware<T>>>) -> Self {
        Self { deque: stack }
    }

    #[inline]
    pub(crate) fn push(&mut self, middleware: Arc<dyn Middleware<T>>) {
        self.deque.push_back(middleware);
    }
}
