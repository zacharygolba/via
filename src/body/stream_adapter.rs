use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::util;
use crate::Error;

/// Convert a `Stream + Send` into an `impl Body`.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<S> {
    /// The `Stream` that we are adapting to an `impl Body`.
    stream: S,
}

impl<S, E> StreamAdapter<S>
where
    S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin,
    Error: From<E>,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S: Unpin> StreamAdapter<S> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut S> {
        // Get a mutable reference to `self`.
        let this = self.get_mut();
        // Get a mutable reference to `self.stream`.
        let ptr = &mut this.stream;

        Pin::new(ptr)
    }
}

impl<S, E> Body for StreamAdapter<S>
where
    S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin,
    Error: From<E>,
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
        util::size_hint_from_stream_for_body(&self.stream)
    }
}
