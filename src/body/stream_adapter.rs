use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
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
    E: Into<Error>,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S, E> Body for StreamAdapter<S>
where
    S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin,
    E: Into<Error>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // Get a mutable reference to `self`.
        let this = self.get_mut();
        // Get a mutable reference to `self.stream`.
        let ptr = &mut this.stream;

        Pin::new(ptr)
            .poll_next(context)
            .map_err(|error| error.into())
    }

    fn size_hint(&self) -> SizeHint {
        util::size_hint_from_stream_for_body(&self.stream)
    }
}
