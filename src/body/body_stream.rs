use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::limit_error::error_from_boxed;
use super::request_body::RequestBody;
use crate::body::HttpBody;
use crate::error::Error;

/// A stream of frames that compose the payload and trailers of a request body.
///
#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: HttpBody<RequestBody>,
}

#[inline]
fn size_hint_as_usize(hint: SizeHint) -> (Option<usize>, Option<usize>) {
    (
        hint.lower().try_into().ok(),
        hint.upper().and_then(|value| value.try_into().ok()),
    )
}

impl BodyStream {
    /// Creates a new `BodyStream` with the provided request body.
    pub(crate) fn new(body: HttpBody<RequestBody>) -> Self {
        Self { body }
    }
}

impl Stream for BodyStream {
    type Item = Result<Frame<Bytes>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.body)
            .poll_frame(context)
            .map_err(error_from_boxed)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match size_hint_as_usize(self.body.size_hint()) {
            (Some(low), high) => (low, high),
            (None, _) => (0, None),
        }
    }
}
