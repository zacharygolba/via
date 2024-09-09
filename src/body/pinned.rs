use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

/// An `!Unpin` version of [`Boxed`](super::Boxed).
#[must_use = "streams do nothing unless polled"]
pub struct Pinned {
    body: Box<dyn Body<Data = Bytes, Error = Error> + Send>,
    _pin: PhantomPinned,
}

impl Pinned {
    /// Creates a new response body from a `!Unpin` body.
    ///
    /// If the body is `Unpin`, use [`Boxed`](super::Boxed) instead.
    ///
    /// This method is marked as unsafe because the caller must ensure that the
    /// provided body is `!Unpin` and properly remains pinned in memory while the
    /// body is polled.
    pub unsafe fn new_unchecked<B>(body: Box<B>) -> Self
    where
        B: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        Self {
            body,
            _pin: PhantomPinned,
        }
    }
}

impl Pinned {
    fn project(self: Pin<&mut Self>) -> Pin<&mut (dyn Body<Data = Bytes, Error = Error> + Send)> {
        unsafe {
            //
            // Safety:
            //
            // The `body` field is `!Unpin` because it may contain a trait object
            // that is `!Unpin`. It is the responsibility of the caller
            // `ResponseBody::pin` to ensure that the body properly remains
            // pinned while the response body is polled.
            //
            Pin::map_unchecked_mut(self, |this| &mut *this.body)
        }
    }
}

impl Body for Pinned {
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
