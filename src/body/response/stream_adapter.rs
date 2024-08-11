use bytes::Bytes;
use futures_core::Stream;
use hyper::body::Frame;
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

/// A stream adapter that converts a stream of `Result<D, E>` into a stream of
/// `Result<Frame<Bytes>>`. This adapter allows for response bodies to be built
/// from virtually any stream that yields data that can be converted into bytes.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    /// The `Stream` that we are adapting to yield `Result<Frame<Bytes>>`.
    stream: T,

    /// This field is used to mark `ResponseBodyStreamAdapter` as `!Unpin`. This
    /// is necessary because `T` may not be `Unpin` and we need to project the
    /// `stream` through a pinned reference to `Self` so it can be polled.
    _pin: PhantomPinned,
}

impl<T, D, E> StreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    pub(crate) fn new(stream: T) -> Self {
        Self {
            stream,
            _pin: PhantomPinned,
        }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // Safety:
        // This block is necessary because we need to project the inner stream
        // through the outer pinned reference. We don't know if `T` is `Unpin`
        // so we need to use `unsafe` to create the pinned reference with
        // `Pin::new_unchecked`.
        unsafe {
            // Get a mutable reference to `self`.
            let this = self.get_unchecked_mut();
            // Get a mutable reference to the `stream` field.
            let stream = &mut this.stream;

            // Return the pinned reference to the `stream` field.
            Pin::new_unchecked(stream)
        }
    }
}

impl<T, D, E> Stream for StreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().poll_next(context) {
            Poll::Ready(Some(Ok(data))) => {
                // Convert the data to bytes.
                let bytes = Bytes::from(data);
                // Wrap the bytes in a data frame.
                let frame = Frame::data(bytes);
                // Yield the data frame.
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(error))) => {
                // An error occurred while reading the stream. Wrap the
                // error with `via::Error`.
                let error = Error::from(error);
                // Yield the wrapped error.
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                // The stream has ended.
                Poll::Ready(None)
            }
            Poll::Pending => {
                // The stream is not ready to yield a frame.
                Poll::Pending
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Get the size hint from the inner stream.
        self.stream.size_hint()
    }
}
