use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::{AnyBody, Boxed, Buffered, Either, Pinned};
use crate::Error;

pub struct ResponseBody {
    body: Either<AnyBody<Buffered>, Pinned>,
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        Self {
            body: Either::Left(AnyBody::new()),
        }
    }
}

impl ResponseBody {
    pub(crate) fn into_inner(self) -> Either<AnyBody<Buffered>, Pinned> {
        self.body
    }
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<AnyBody<Buffered>, Pinned>> {
        unsafe {
            //
            // Safety:
            //
            // The `body` field is `!Unpin` because it may contain a Pinned body.
            // While we are unable to express the negative trait bound in the
            // Pinned::new constructor, it is the only place where a
            // Pinned body can be created and the `!Unpin` requirement is
            // mentioned in the documentation.
            //
            // Regardless, we are not moving the value out of the struct, so it
            // is safe to project the field.
            //
            Pin::map_unchecked_mut(self, |this| &mut this.body)
        }
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Boxed> for ResponseBody {
    fn from(boxed: Boxed) -> Self {
        Self {
            body: Either::Left(AnyBody::Boxed(boxed)),
        }
    }
}

impl From<Pinned> for ResponseBody {
    fn from(pinned: Pinned) -> Self {
        Self {
            body: Either::Right(pinned),
        }
    }
}

impl<T> From<T> for ResponseBody
where
    Buffered: From<T>,
{
    fn from(value: T) -> Self {
        let buffered = Buffered::from(value);

        Self {
            body: Either::Left(AnyBody::Inline(buffered)),
        }
    }
}
