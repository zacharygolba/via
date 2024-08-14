use bytes::Bytes;
use futures_core::{ready, Stream};
use hyper::body::{Body, Incoming};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::size_hint;
use crate::Result;

/// A data stream of bytes that compose the body of a request.
#[must_use = "streams do nothing unless polled"]
pub struct BodyDataStream {
    body: Box<Incoming>,
}

impl BodyDataStream {
    /// Creates a new `BodyDataStream` with the provided request body.
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

impl Stream for BodyDataStream {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        // Get a pinned mutable reference to `self.body`.
        let mut body = self.project();

        loop {
            // Reborrow the pinned mutable reference to `self.body` and poll
            // the stream for the next frame. If the stream is not ready,
            // return early.
            return match ready!(body.as_mut().poll_frame(context)) {
                // A frame was successfully polled from the stream. Attempt
                // to pull the data out of the frame.
                Some(Ok(frame)) => match frame.into_data() {
                    // The frame is a data frame. Return `Ready`.
                    Ok(data) => Poll::Ready(Some(Ok(data))),
                    // The frame is trailers. Ignore them and continue.
                    Err(_) => continue,
                },
                // An error occurred while polling the stream. Convert the
                // error to a `via::Error` and return.
                Some(Err(error)) => Poll::Ready(Some(Err(error.into()))),
                // The stream has been exhausted.
                None => Poll::Ready(None),
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Delegate the call to `self.body` to get a `SizeHint` and use the
        // helper function to adapt the returned `SizeHint` to a tuple that
        // contains the lower and upper bound of the stream.
        size_hint::from_body_for_stream(&self.body)
    }
}
