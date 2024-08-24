use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::size_hint;
use crate::{Error, Result};

/// A streaming response body. Streaming bodies are useful when the body data is
/// too large to be buffered in memory or when the each frame of the body needs
/// to be processed as it is received.
pub struct Streaming {
    /// A trait object that implements `Stream` that we'll poll to get the next
    /// frame of the response body in the `poll_frame` method.
    stream: Box<dyn Stream<Item = Result<Frame<Bytes>>> + Send>,
}

impl Streaming {
    /// Creates a new `Streaming` response body with the provided `stream`.
    pub fn new<T>(stream: Box<T>) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>>> + Send + 'static,
    {
        Self { stream }
    }
}

impl Streaming {
    /// Returns a pinned mutable reference to the `stream` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut (dyn Stream<Item = Result<Frame<Bytes>>> + Send)> {
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
            self.map_unchecked_mut(|this| &mut *this.stream)
        }
    }
}

impl Body for Streaming {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_next(context)
    }

    fn size_hint(&self) -> SizeHint {
        size_hint::from_stream_for_body(&*self.stream)
    }
}
