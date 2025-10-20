use std::collections::VecDeque;
use std::sync::Arc;

use crate::middleware::{BoxFuture, Middleware};
use crate::request::Request;

pub struct Next<T = ()> {
    deque: VecDeque<Arc<dyn Middleware<T>>>,
}

impl<T> Next<T> {
    pub fn call(mut self, request: Request<T>) -> BoxFuture {
        match self.deque.pop_front() {
            Some(middleware) => middleware.call(request, self),
            None => Box::pin(async { Err(crate::raise!(404)) }),
        }
    }
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new(deque: VecDeque<Arc<dyn Middleware<T>>>) -> Self {
        Self { deque }
    }
}
