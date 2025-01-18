use bytes::Bytes;
use http_body::{self, Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::body_reader::{BodyData, BodyReader};
use super::body_stream::BodyStream;
use super::limit_error::LimitError;
use crate::body::{BoxBody, HttpBody};
use crate::error::{BoxError, Error};

#[derive(Debug)]
pub struct RequestBody {
    remaining: usize,
    body: Option<BoxBody<Bytes, hyper::Error>>,
}

impl RequestBody {
    #[inline]
    pub(crate) fn new(remaining: usize, body: Option<BoxBody<Bytes, hyper::Error>>) -> Self {
        Self { remaining, body }
    }
}

impl HttpBody<RequestBody> {
    pub fn stream(self) -> BodyStream {
        BodyStream::new(self)
    }

    pub async fn read_to_end(self) -> Result<BodyData, Error> {
        BodyReader::new(self).await
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
        let body = match &mut this.body {
            Some(body) => body,
            None => return Poll::Ready(None),
        };

        match Pin::new(body).poll_frame(context)? {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(frame)) => {
                if let Some(chunk) = frame.data_ref() {
                    let len = chunk.len();

                    if this.remaining < len {
                        let error = Box::new(LimitError);
                        return Poll::Ready(Some(Err(error)));
                    }

                    this.remaining -= len;
                }

                Poll::Ready(Some(Ok(frame)))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.body {
            Some(body) => body.is_end_stream(),
            None => true,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.body {
            Some(body) => body.size_hint(),
            None => SizeHint::new(),
        }
    }
}
