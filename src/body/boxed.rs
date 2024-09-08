use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

/// A struct that wraps a `Pin<Box<dyn Body>>`.
#[must_use = "streams do nothing unless polled"]
pub struct Boxed {
    body: Pin<Box<dyn Body<Data = Bytes, Error = Error> + Send>>,
}

impl Boxed {
    pub fn new<T>(body: Box<T>) -> Self
    where
        T: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        Self {
            body: Box::into_pin(body),
        }
    }
}

impl Body for Boxed {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::as_mut(&mut self.body).poll_frame(context)
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
