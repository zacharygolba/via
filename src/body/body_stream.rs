use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{util, BoxBody};
use crate::error::Error;

/// A stream of frames that compose the payload and trailers of a request body.
///
#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: BoxBody,
}

impl BodyStream {
    /// Creates a new `BodyStream` with the provided request body.
    pub(crate) fn new(body: BoxBody) -> Self {
        Self { body }
    }
}

impl BodyStream {
    #[inline]
    fn project(self: Pin<&mut Self>) -> Pin<&mut BoxBody> {
        Pin::new(&mut self.get_mut().body)
    }
}

impl Stream for BodyStream {
    type Item = Result<Frame<Bytes>, Error>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        self.project().poll_frame(context).map_err(util::map_err)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let hint = self.body.size_hint();

        (
            hint.lower().try_into().expect("overflow would occur"),
            hint.upper()
                .map(|upper| upper.try_into().expect("overflow would occur")),
        )
    }
}
