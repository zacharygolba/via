use bytes::Bytes;
use futures_core::Stream;
use hyper::body::Frame;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// A stream adapter that converts a stream of `Result<D, E>` into a stream of
/// `Result<Frame<Bytes>>`. This adapter allows for response bodies to be built
/// from virtually any stream that yields data that can be converted into bytes.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    /// The `Stream` that we are adapting to yield `Result<Frame<Bytes>>`.
    stream: S,
}

impl<S, D, E> StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    pub(crate) fn new(stream: S) -> Self {
        Self { stream }
    }
}

impl<S, D, E> StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    /// Returns a pinned mutable reference to the `stream` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut S> {
        unsafe {
            //
            // Safety:
            //
            // The `stream` field is a trait object that implements `Stream` and
            // may be `!Unpin`. Therefore we have to use `Pin::map_unchecked_mut`
            // to wrap the mutable reference to `stream` in a pinned reference.
            // This is safe because we do not move `stream` or any data owned by
            // the `stream` field out of the pinned mutable reference.
            //
            self.map_unchecked_mut(|this| &mut this.stream)
        }
    }
}

impl<S, D, E> Stream for StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .poll_next(context)
            .map_ok(|data| Frame::data(data.into()))
            .map_err(|error| error.into())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Get the size hint from the `stream` field.
        self.stream.size_hint()
    }
}
