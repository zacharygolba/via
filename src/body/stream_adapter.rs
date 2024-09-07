use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::size_hint;
use crate::Error;

/// A stream adapter that converts a stream of `Result<D, E>` into a stream of
/// `Result<Frame<Bytes>>`. This adapter allows for response bodies to be built
/// from virtually any stream that yields data that can be converted into bytes.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<S> {
    /// The `Stream` that we are adapting to yield `Result<Frame<Bytes>>`.
    stream: S,
}

impl<S, E> StreamAdapter<S>
where
    S: Stream<Item = Result<Frame<Bytes>, E>> + Send,
    E: Into<Error>,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S, E> StreamAdapter<S>
where
    S: Stream<Item = Result<Frame<Bytes>, E>> + Send,
    E: Into<Error>,
{
    /// Returns a pinned mutable reference to the `stream` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe {
            //
            // Safety:
            //
            // TODO: Add safety comment.
            //
            self.map_unchecked_mut(|this| &mut this.stream)
        }
    }
}

impl<S, E> Body for StreamAdapter<S>
where
    S: Stream<Item = Result<Frame<Bytes>, E>> + Send,
    E: Into<Error>,
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
        size_hint::from_stream_for_body(&self.stream)
    }
}
