use bytes::Bytes;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BodyStream;
use crate::Result;

#[must_use = "streams do nothing unless polled"]
pub struct BodyDataStream {
    stream: BodyStream,
}

impl BodyDataStream {
    pub(crate) fn new(stream: BodyStream) -> Self {
        Self { stream }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut BodyStream> {
        // Get a mutable reference to self.
        let this = self.get_mut();
        // Get a mutable reference to the `stream` field.
        let stream = &mut this.stream;

        // Return the pinned reference to the `stream` field.
        Pin::new(stream)
    }
}

impl Stream for BodyDataStream {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        match self.project().poll_next(context) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Ok(bytes) = frame.into_data() {
                    // The frame is a data frame. Return it.
                    Poll::Ready(Some(Ok(bytes)))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(error))) => {
                // An error occurred while polling the stream.
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                // No more frames.
                Poll::Ready(None)
            }
            Poll::Pending => {
                // Wait for the next frame.
                Poll::Pending
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}
