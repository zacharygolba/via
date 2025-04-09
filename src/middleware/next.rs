use std::collections::VecDeque;
use std::sync::Arc;

use super::middleware::{BoxFuture, Middleware};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    deque: VecDeque<Arc<dyn Middleware<T>>>,
}

impl<T> Next<T> {
    pub fn call(mut self, request: Request<T>) -> BoxFuture {
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
    pub(crate) fn new(deque: VecDeque<Arc<dyn Middleware<T>>>) -> Self {
        Self { deque }
    }

    #[inline]
    pub(crate) fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Arc<dyn Middleware<T>>>,
    {
        self.deque.extend(iter);
    }
}
