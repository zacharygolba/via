use std::sync::Weak;

use super::middleware::{FutureResponse, Middleware};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    stack: Vec<Weak<dyn Middleware<T>>>,
}

impl<T> Next<T> {
    #[inline]
    pub fn call(mut self, request: Request<T>) -> FutureResponse {
        match self.stack.pop().map(|weak| Weak::upgrade(&weak)) {
            Some(Some(middleware)) => {
                // Call middleware to get a response.
                middleware.call(request, self)
            }
            Some(None) => {
                // Placeholder for tracing...
                Box::pin(async {
                    let message = "internal server error".to_owned();
                    Err(Error::internal_server_error(message.into()))
                })
            }
            None => {
                // Respond with a 404 Not Found.
                Box::pin(async {
                    let message = "not found".to_owned();
                    Err(Error::not_found(message.into()))
                })
            }
        }
    }
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new(stack: Vec<Weak<dyn Middleware<T>>>) -> Self {
        Self { stack }
    }

    #[inline]
    pub(crate) fn push(&mut self, middleware: Weak<dyn Middleware<T>>) {
        self.stack.push(middleware)
    }
}
