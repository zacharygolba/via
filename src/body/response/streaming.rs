use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{body::size_hint, Error, Result};

/// A streaming response body. Streaming bodies are useful when the body data is
/// too large to be buffered in memory or when the each frame of the body needs
/// to be processed as it is received.
pub struct Streaming {
    /// A trait object that implements `Stream` that we'll poll to get the next
    /// frame of the response body in the `poll_frame` method.
    stream: Box<dyn Stream<Item = Result<Bytes>> + Send>,

    /// This field is used to mark `Streaming` as `!Unpin`. This is necessary
    /// because `self.stream` may not be `Unpin` and we need to project the
    /// `self.stream` through a pinned reference to `Self` so it can be
    /// polled.
    _pin: PhantomPinned,
}

impl Streaming {
    pub fn new<T>(stream: T) -> Self
    where
        T: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        Self {
            stream: Box::new(stream),
            _pin: PhantomPinned,
        }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut (dyn Stream<Item = Result<Bytes>> + Send)> {
        // Get a mutable reference to `self`.
        let this = unsafe {
            // Safety:
            // TODO: Add safety explanation.
            self.get_unchecked_mut()
        };
        // Get a mutable reference to the `stream` field.
        let ptr = &mut *this.stream;

        // Return a pinned reference to `stream` field.
        unsafe {
            // Safety:
            // TODO: Add safety explanation.
            Pin::new_unchecked(ptr)
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
        self.project().poll_next(context).map_ok(Frame::data)
    }

    fn is_end_stream(&self) -> bool {
        false
    }

    fn size_hint(&self) -> SizeHint {
        size_hint::from_stream_for_body(&*self.stream)
    }
}
