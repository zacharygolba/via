use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{body::size_hint, Error, Result};

/// A type alias for the trait object that represents the stream of frames that
/// compose the body of `Streaming`. This type alias exists to simply the type
/// signatures in this module.
type StreamBody = dyn Stream<Item = Result<Frame<Bytes>>> + Send;

/// A streaming response body. Streaming bodies are useful when the body data is
/// too large to be buffered in memory or when the each frame of the body needs
/// to be processed as it is received.
pub struct Streaming {
    /// A trait object that implements `Stream` that we'll poll to get the next
    /// frame of the response body in the `poll_frame` method.
    stream: Box<StreamBody>,

    /// This field is used to mark `Streaming` as `!Unpin`. This is necessary
    /// because `self.stream` may be `!Unpin` and we need to project `stream`
    /// through a pinned reference to `Self` so it can be polled for the next
    /// frame.
    _pin: PhantomPinned,
}

impl Streaming {
    /// Creates a new `Streaming` response body with the provided `stream`.
    pub fn new<T>(stream: Box<T>) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>>> + Send + 'static,
    {
        Self {
            stream,
            _pin: PhantomPinned,
        }
    }

    /// Returns a pinned mutable reference to the `stream` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut StreamBody> {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // The `body` field may contain a `Streaming` response body which is
            // not `Unpin`. We need to project the body field through a pinned
            // reference to `Self` so that it can be polled in the `poll_frame`
            // method. This is safe because no data is moved out of `self`.
            self.get_unchecked_mut()
        };
        // Get a mutable reference to the `stream` field.
        let stream = &mut *this.stream;

        // Return the pinned mutable reference to the `stream` field.
        unsafe {
            //
            // Safety:
            //
            // The `stream` field is a trait object that implements `Stream` and
            // may be `!Unpin`. Therefore we have to use `Pin::new_unchecked` to
            // wrap the mutable reference to `stream` in a pinned reference. This
            // is safe because we are not moving the value or any data owned by
            // the `stream` field out of the pinned mutable reference.
            Pin::new_unchecked(stream)
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

    fn is_end_stream(&self) -> bool {
        false
    }

    fn size_hint(&self) -> SizeHint {
        size_hint::from_stream_for_body(&*self.stream)
    }
}
