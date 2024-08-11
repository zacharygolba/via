use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Incoming};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{body::size_hint, Error, Result};

#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    pub(super) body: Box<Incoming>,
}

impl BodyStream {
    pub(crate) fn new(body: Box<Incoming>) -> Self {
        Self { body }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut Incoming> {
        // Get a mutable reference to self.
        let this = self.get_mut();
        // Get a mutable reference to the `body` field.
        let body = &mut *this.body;

        // Return the pinned reference to the `body` field.
        Pin::new(body)
    }
}

impl Stream for BodyStream {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        match self.project().poll_frame(context) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Ok(bytes) = frame.into_data() {
                    // The frame is a data frame. Return it.
                    Poll::Ready(Some(Ok(bytes)))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(error))) => {
                let error = Error::from(error);
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
        // Delegate the call to the stream to get a `SizeHint` and use the
        // helper function to adapt the returned `SizeHint` to a tuple that
        // contains the lower and upper bound of the stream.
        size_hint::from_body_for_stream(&self.body)
    }
}
