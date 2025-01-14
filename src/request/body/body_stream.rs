use bytes::Bytes;
use futures_core::Stream;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::length_limit_error::error_from_boxed;
use super::request_body::RequestBody;
use crate::error::Error;

/// A stream of frames that compose the body and trailers of a request.
///
#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: RequestBody,
}

fn size_hint_as_usize(hint: SizeHint) -> (Option<usize>, Option<usize>) {
    (
        hint.lower().try_into().ok(),
        hint.upper().and_then(|value| value.try_into().ok()),
    )
}

impl BodyStream {
    /// Creates a new `BodyStream` with the provided request body.
    pub(crate) fn new(body: RequestBody) -> Self {
        Self { body }
    }
}

impl Stream for BodyStream {
    type Item = Result<Frame<Bytes>, Error>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        Pin::new(&mut this.body)
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
