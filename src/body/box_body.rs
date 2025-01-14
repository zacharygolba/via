use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::BoxError;

/// Converts an `impl Body` to a type-erased trait object.
///
#[must_use = "streams do nothing unless polled"]
pub struct BoxBody {
    body: Pin<Box<dyn Body<Data = Bytes, Error = BoxError> + Send + Sync>>,
}

impl BoxBody {
    pub fn new<T>(body: T) -> Self
    where
        T: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self {
            body: Box::pin(body),
        }
    }
}

impl Body for BoxBody {
    type Data = Bytes;
    type Error = BoxError;

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

impl Debug for BoxBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxBody").finish()
    }
}
