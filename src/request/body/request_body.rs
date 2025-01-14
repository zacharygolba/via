use bytes::Bytes;
use http_body::{self, Body, Frame, SizeHint};
use hyper::body::Incoming;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::body_reader::BodyReader;
use super::body_stream::BodyStream;
use super::length_limit_error::LengthLimitError;
use crate::body::{BoxBody, HttpBody};
use crate::error::BoxError;

#[derive(Debug)]
pub struct HyperBody {
    body: Incoming,
}

#[derive(Debug)]
pub struct RequestBody {
    remaining: usize,
    body: HttpBody<HyperBody>,
}

impl Body for HyperBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        Pin::new(&mut this.body)
            .poll_frame(context)
            .map_err(|error| error.into())
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl RequestBody {
    pub fn stream(self) -> BodyStream {
        BodyStream::new(self)
    }

    pub fn read_to_end(self) -> BodyReader {
        BodyReader::new(self)
    }
}

impl RequestBody {
    #[inline]
    pub(crate) fn new(remaining: usize, body: Incoming) -> Self {
        Self {
            remaining,
            body: HttpBody::Inline(HyperBody { body }),
        }
    }

    #[inline]
    pub(crate) fn map<F>(self, map: F) -> Self
    where
        F: FnOnce(HttpBody<HyperBody>) -> BoxBody,
    {
        Self {
            remaining: self.remaining,
            body: HttpBody::Box(map(self.body)),
        }
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        match Pin::new(&mut this.body).poll_frame(context)? {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(frame)) => {
                if let Some(data) = frame.data_ref() {
                    let frame_len = data.len();

                    if this.remaining < frame_len {
                        let error = Box::new(LengthLimitError);
                        return Poll::Ready(Some(Err(error)));
                    }

                    this.remaining -= frame_len;
                }

                Poll::Ready(Some(Ok(frame)))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
