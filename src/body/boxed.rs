use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

/// A struct that turns an `impl Body + Send + !Unpin` into a type erased body.
///
/// This struct should only be used with `!Unpin` bodies.
///
#[must_use = "streams do nothing unless polled"]
pub struct NotUnpinBoxBody {
    body: Box<dyn Body<Data = Bytes, Error = Error> + Send>,
    _pin: PhantomPinned,
}

impl NotUnpinBoxBody {
    /// Creates a new body from a `dyn Body + Send + !Unpin`.
    ///
    pub fn new<B>(body: Box<B>) -> Self
    where
        B: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        Self {
            body,
            _pin: PhantomPinned,
        }
    }
}

impl NotUnpinBoxBody {
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

impl Body for NotUnpinBoxBody {
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

impl Debug for NotUnpinBoxBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotUnpinBoxBody").finish()
    }
}
