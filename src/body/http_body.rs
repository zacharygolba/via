use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{BoxBody, RequestBody};
use crate::error::BoxError;

/// The body of a request or response.
///
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

enum HttpBodyProjection<'a, T> {
    Original(Pin<&'a mut T>),
    Mapped(Pin<&'a mut BoxBody>),
}

impl<T> HttpBody<T> {
    fn project(self: Pin<&mut Self>) -> HttpBodyProjection<T> {
        unsafe {
            match self.get_unchecked_mut() {
                Self::Original(body) => HttpBodyProjection::Original(Pin::new_unchecked(body)),
                Self::Mapped(body) => HttpBodyProjection::Mapped(Pin::new(body)),
            }
        }
    }
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
    T: Body<Data = Bytes, Error = BoxError>,
{
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            HttpBodyProjection::Original(body) => body.poll_frame(cx),
            HttpBodyProjection::Mapped(body) => body.poll_frame(cx),
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

impl From<RequestBody> for HttpBody<RequestBody> {
    #[inline]
    fn from(body: RequestBody) -> Self {
        Self::Original(body)
    }
}
