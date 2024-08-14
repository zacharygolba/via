use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, Incoming};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::body::size_hint;
use crate::Result;

/// A stream of frames that compose the body and trailers of a request.
#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: Box<Incoming>,
}

impl BodyStream {
    /// Creates a new `BodyStream` with the provided request body.
    pub(crate) fn new(body: Box<Incoming>) -> Self {
        Self { body }
    }

    /// Returns a pinned mutable reference to the `body` field. This method is
    /// used to project the `body` field through a pinned reference to `Self`
    /// so it can be polled for the next frame.
    fn project(self: Pin<&mut Self>) -> Pin<&mut Incoming> {
        // Get a mutable reference to self.
        let this = self.get_mut();
        // Get a mutable reference to the `body` field by dereferencing
        // `Box<Incoming>` to `&mut Incoming`.
        let body = &mut *this.body;

        // Return the pinned mutable reference to the `body` field.
        Pin::new(body)
    }
}

impl Stream for BodyStream {
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        // Delegate the call to `self.body` to poll the next frame. If an error
        // occurs while polling the body, map the error to a `via::Error`.
        self.project()
            .poll_frame(context)
            .map_err(|error| error.into())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Delegate the call to `self.body` to get a `SizeHint` and use the
        // helper function to adapt the returned `SizeHint` to a tuple that
        // contains the lower and upper bound of the stream.
        size_hint::from_body_for_stream(&self.body)
    }
}
