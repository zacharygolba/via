use std::future::Future;
use std::sync::Arc;

use super::middleware::{FutureResponse, Middleware};
use crate::body::{HttpBody, ResponseBody};
use crate::error::Error;
use crate::request::Request;

pub struct Next<T = ()> {
    stack: Vec<Arc<dyn Middleware<T>>>,
}

impl<T> Next<T> {
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

impl<T> Next<T> {
    pub(crate) fn new(stack: Vec<Arc<dyn Middleware<T>>>) -> Self {
        Self { stack }
    }

    pub(crate) fn run(
        self,
        request: Request<T>,
    ) -> impl Future<Output = http::Response<HttpBody<ResponseBody>>> {
        let call = self.call(request);

        async {
            // If `future` resolves with an error, generate a response from it.
            // Then, unwrap the http::Response from via::Response and return.
            call.await.unwrap_or_else(|error| error.into()).into_inner()
        }
    }
}
