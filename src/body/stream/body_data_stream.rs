use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BodyStream;
use crate::Result;

/// A data stream of bytes that compose the body of a request.
#[must_use = "streams do nothing unless polled"]
pub struct BodyDataStream {
    stream: BodyStream,
}

impl BodyDataStream {
    /// Creates a new `BodyDataStream` with the provided `BodyStream`.
    pub(crate) fn new(stream: BodyStream) -> Self {
        Self { stream }
    }
}

impl Stream for BodyDataStream {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        // Get a mutable reference to `Self`.
        let this = self.get_mut();
        // Get a mutable reference to the `stream` field.
        let stream = &mut this.stream;
        // Create a new `Pin` around the mtuable reference to the `stream` field.
        let mut pin = Pin::new(stream);

        loop {
            // Reborrow the `pin` so we can poll the `stream` field for the next
            // frame. If the stream is not ready, return early.
            return match pin.as_mut().poll_next(context) {
                // A frame was successfully polled from the stream. Attempt
                // to pull the data out of the frame.
                Poll::Ready(Some(Ok(frame))) => match frame.into_data() {
                    // The frame is a data frame. Return `Ready`.
                    Ok(data) => Poll::Ready(Some(Ok(data))),
                    // The frame is trailers. Ignore them and continue.
                    Err(_) => continue,
                },
                // An error occurred while polling the stream. Return `Ready`
                // with the error.
                Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
                // The stream has been exhausted.
                Poll::Ready(None) => Poll::Ready(None),
                // Wait for the next frame.
                Poll::Pending => Poll::Pending,
            };
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Return the size hint from the `BodyStream` at `self.stream`.
        self.stream.size_hint()
    }
}
