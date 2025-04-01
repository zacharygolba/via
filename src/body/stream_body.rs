use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::DynError;

/// Converts an `impl Stream` to an `impl Body`.
///
#[must_use = "streams do nothing unless polled"]
pub struct StreamBody<T> {
    stream: T,
}

impl<T> StreamBody<T> {
    #[inline]
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T> StreamBody<T> {
    #[inline]
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|this| &mut this.stream) }
    }
}

impl<T> Body for StreamBody<T>
where
    T: Stream<Item = Result<Bytes, DynError>> + Send + Sync,
{
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_next(context).map_ok(Frame::data)
    }

    fn is_end_stream(&self) -> bool {
        false
    }

    fn size_hint(&self) -> SizeHint {
        let mut hint = SizeHint::new();
        let existing = self.stream.size_hint();

        match (
            existing.0.try_into().ok(),
            existing.1.and_then(|upper| upper.try_into().ok()),
        ) {
            (None, _) => {}
            (Some(lower), None) => hint.set_lower(lower),
            (Some(lower), Some(upper)) => {
                hint.set_lower(lower);
                hint.set_upper(upper);
            }
        }

        hint
    }
}

impl<T> Debug for StreamBody<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("StreamBody").finish()
    }
}
