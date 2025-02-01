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
        let next = {
            let stack = &mut self.stack;

            #[allow(clippy::never_loop)]
            loop {
                if let Some(weak) = stack.pop() {
                    break weak;
                }

                return Box::pin(async {
                    let message = "not found".to_owned();
                    Err(Error::not_found(message.into()))
                });
            }
        };

        match Weak::upgrade(&next) {
            Some(middleware) => middleware.call(request, Self { ..self }),
            None => Box::pin(async {
                let message = "internal server error".to_owned();
                Err(Error::internal_server_error(message.into()))
            }),
        }
    }
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new(stack: Vec<Weak<dyn Middleware<T>>>) -> Self {
        Self { stack }
    }
}
