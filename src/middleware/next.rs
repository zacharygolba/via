use std::collections::VecDeque;
use via_router::Binding;

use super::middleware::{FutureResponse, Middleware};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    offset: usize,
    stack: VecDeque<Binding<Box<dyn Middleware<T>>>>,
}

impl<T> Next<T> {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            offset: 0,
            stack: VecDeque::new(),
        }
    }

    #[inline]
    pub(crate) fn stack_mut(&mut self) -> &mut VecDeque<Binding<Box<dyn Middleware<T>>>> {
        &mut self.stack
    }

    pub fn call(mut self, request: Request<T>) -> FutureResponse {
        loop {
            if self.offset > self.stack.len() {
                return Box::pin(async {
                    let message = "not found".to_owned();
                    Err(Error::not_found(message.into()))
                });
            }

            if let Some(binding) = self.stack.get_mut(self.offset) {
                let key = match binding.next() {
                    Some(key) => key,
                    None => {
                        self.offset += 1;
                        continue;
                    }
                };

                return binding.get(key).unwrap().call(request, self);
            }

            self.offset += 1;
        }
    }
}
