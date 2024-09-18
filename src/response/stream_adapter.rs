use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::size_hint;

/// Convert a `Stream + Send` into an `impl Body`.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<T> {
    /// The `Stream` that we are adapting to an `impl Body`.
    stream: T,
}

impl<T, E> StreamAdapter<T>
where
    T: Stream<Item = Result<Frame<Bytes>, E>> + Send,
{
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T> StreamAdapter<T> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        //
        // Safety:
        //
        // The stream field may contain a type that is !Unpin. We need a pinned
        // reference to the stream field in order to call poll_next in the
        // implementation of Body::poll_frame. This is safe because the stream
        // field is never moved out of self.
        //
        unsafe { Pin::map_unchecked_mut(self, |this| &mut this.stream) }
    }
}

impl<T, E> Body for StreamAdapter<T>
where
    T: Stream<Item = Result<Frame<Bytes>, E>> + Send,
{
    type Data = Bytes;
    type Error = E;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_next(context)
    }

    fn size_hint(&self) -> SizeHint {
        let hint = self.stream.size_hint();
        size_hint::from_stream_for_body(hint)
    }
}
