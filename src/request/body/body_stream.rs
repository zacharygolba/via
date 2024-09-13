use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame};
use hyper::body::Incoming;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::util::size_hint;
use crate::body::AnyBody;
use crate::Result;

/// A stream of frames that compose the body and trailers of a request.
#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: AnyBody<Incoming>,
}

impl BodyStream {
    /// Creates a new `BodyStream` with the provided request body.
    pub(crate) fn new(body: AnyBody<Incoming>) -> Self {
        Self { body }
    }
}

impl Stream for BodyStream {
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        // Get a mutable reference to `Self`.
        let this = self.get_mut();
        // Get a mutable reference to the `body` field.
        let body = &mut this.body;

        Pin::new(body).poll_frame(context)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Delegate the call to `self.body` to get a `SizeHint` and use the
        // helper function to adapt the returned `SizeHint` to a tuple that
        // contains the lower and upper bound of the stream.
        size_hint::from_body_for_stream(&self.body)
    }
}
