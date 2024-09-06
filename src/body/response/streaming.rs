use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::size_hint;
use crate::Error;

/// A streaming response body. Streaming bodies are useful when the body data is
/// too large to be buffered in memory or when the each frame of the body needs
/// to be processed as it is received.
pub struct Streaming {
    /// A trait object that implements `Stream` that we'll poll to get the next
    /// frame of the response body in the `poll_frame` method.
    stream: Pin<Box<dyn Stream<Item = Result<Frame<Bytes>, Error>> + Send>>,
}

impl Streaming {
    /// Creates a new `Streaming` response body with the provided `stream`.
    pub fn new<T>(stream: Pin<Box<T>>) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>, Error>> + Send + 'static,
    {
        Self { stream }
    }
}

impl Body for Streaming {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.stream.as_mut().poll_next(context)
    }

    fn size_hint(&self) -> SizeHint {
        size_hint::from_stream_for_body(&*self.stream)
    }
}
