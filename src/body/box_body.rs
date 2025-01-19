use bytes::{Buf, Bytes};
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::BoxError;

/// Wrap an `impl Body` in a type-erased trait object.
///
#[must_use = "streams do nothing unless polled"]
pub struct BoxBody<D = Bytes, E = BoxError> {
    body: Pin<Box<dyn Body<Data = D, Error = E> + Send + Sync>>,
}

impl<D, E> BoxBody<D, E> {
    pub fn new<T>(body: T) -> Self
    where
        T: Body<Data = D, Error = E> + Send + Sync + 'static,
    {
        Self {
            body: Box::pin(body),
        }
    }
}

impl<D, E> Debug for BoxBody<D, E> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("BoxBody").finish()
    }
}

impl<D: Buf, E> Body for BoxBody<D, E> {
    type Data = D;
    type Error = E;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.body.as_mut().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
