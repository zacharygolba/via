use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::size_hint;
use crate::error::Error;

/// Convert a `Stream + Send` into an `impl Body`.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<T> {
    /// The `Stream` that we are adapting to an `impl Body`.
    stream: T,
}

impl<T, E> StreamAdapter<T>
where
    T: Stream<Item = Result<Frame<Bytes>, E>> + Send + Sync + Unpin,
{
    pub fn new(stream: T) -> Self {
        Self { stream }
    }
}

impl<T: Unpin> StreamAdapter<T> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        let this = self.get_mut();
        let ptr = &mut this.stream;

        Pin::new(ptr)
    }
}

impl<T, E> Body for StreamAdapter<T>
where
    T: Stream<Item = Result<Frame<Bytes>, E>> + Send + Sync + Unpin,
    Error: From<E>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project()
            .poll_next(context)
            .map_err(|error| error.into())
    }

    fn size_hint(&self) -> SizeHint {
        let hint = self.stream.size_hint();
        size_hint::from_stream_for_body(hint)
    }
}
