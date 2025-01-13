use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BoxBody;
use crate::BoxError;

/// A sum type that can represent any
/// [Request](crate::Request)
/// or
/// [Response](crate::Response)
/// body.
///
#[non_exhaustive]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub enum HttpBody<T> {
    /// A statically dispatched `impl Body + Send + Sync`.
    ///
    Inline(T),

    /// A dynamically dispatched `dyn Body + Send + Sync`.
    ///
    Box(BoxBody),
}

impl<T> Body for HttpBody<T>
where
    T: Body<Data = Bytes, Error = BoxError> + Unpin,
{
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            Self::Inline(ptr) => Pin::new(ptr).poll_frame(cx),
            Self::Box(ptr) => Pin::new(ptr).poll_frame(cx),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Inline(body) => body.is_end_stream(),
            Self::Box(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Inline(body) => body.size_hint(),
            Self::Box(body) => body.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    fn from(body: BoxBody) -> Self {
        Self::Box(body)
    }
}
