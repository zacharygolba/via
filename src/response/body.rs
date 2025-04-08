use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use http_body_util::Either;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::{BoxBody, BufferBody};
use crate::error::DynError;

/// A buffered `impl Body` that is read in `8KB` chunks.
///
#[derive(Debug)]
pub struct ResponseBody {
    pub(super) inner: Either<BufferBody, BoxBody>,
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<BufferBody, BoxBody>> {
        unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().inner) }
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl Default for ResponseBody {
    #[inline]
    fn default() -> Self {
        Self {
            inner: Either::Left(Default::default()),
        }
    }
}

impl From<BoxBody> for ResponseBody {
    #[inline]
    fn from(body: BoxBody) -> Self {
        Self {
            inner: Either::Right(body),
        }
    }
}

impl<T> From<T> for ResponseBody
where
    BufferBody: From<T>,
{
    #[inline]
    fn from(body: T) -> Self {
        Self {
            inner: Either::Left(BufferBody::from(body)),
        }
    }
}
