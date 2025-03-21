use bytes::Bytes;
use http_body::{self, Body, Frame, SizeHint};
use hyper::body::Incoming;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::limit_error::LimitError;
use crate::error::DynError;

/// A length-limited `impl Body`. The default limit is `10MB`.
///
/// The maximum length can be configured with
/// [`Server::max_request_size`](crate::Server::max_request_size).
///
#[derive(Debug)]
pub struct RequestBody {
    remaining: usize,
    body: Incoming,
}

impl RequestBody {
    #[inline]
    pub(crate) fn new(remaining: usize, body: Incoming) -> Self {
        Self { remaining, body }
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let frame = match Pin::new(&mut self.body).poll_frame(context)? {
            Poll::Ready(Some(next)) => next,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => return Poll::Pending,
        };

        if let Some(next) = frame.data_ref() {
            let len = next.len();

            if self.remaining < len {
                let error = Box::new(LimitError);
                return Poll::Ready(Some(Err(error)));
            }

            self.remaining -= len;
        }

        Poll::Ready(Some(Ok(frame)))
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
