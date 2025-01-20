use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BoxBody;
use crate::error::BoxError;

/// The body of a request or response.
///
#[non_exhaustive]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub enum HttpBody<T> {
    /// The original body of a request or response.
    ///
    Original(T),

    /// A boxed request or response body that was returned from a map function.
    ///
    Mapped(BoxBody),
}

impl<T> HttpBody<T>
where
    T: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
{
    #[inline]
    pub fn boxed(self) -> BoxBody {
        match self {
            Self::Original(body) => BoxBody::new(body),
            Self::Mapped(body) => body,
        }
    }
}

impl<T> Body for HttpBody<T>
where
    T: Body<Data = Bytes, Error = BoxError> + Unpin,
{
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            Self::Original(ptr) => Pin::new(ptr).poll_frame(context),
            Self::Mapped(ptr) => Pin::new(ptr).poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Original(body) => body.is_end_stream(),
            Self::Mapped(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Original(body) => body.size_hint(),
            Self::Mapped(body) => body.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    fn from(body: BoxBody) -> Self {
        Self::Mapped(body)
    }
}
