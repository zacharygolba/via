use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::tee_body::TeeBody;
use super::RequestBody;
use crate::error::DynError;

/// A type erased, dynamically dispatched [`Body`].
///
pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, DynError>;

/// The body of a request or response.
///
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub enum HttpBody<T> {
    /// The original body of a request or response.
    ///
    Inline(T),

    /// A type erased, dynamically dispatched [`Body`].
    ///
    Dyn(BoxBody),

    ///
    ///
    Tee(TeeBody),
}

impl<T> Body for HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError> + Unpin,
{
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.get_mut() {
            Self::Inline(body) => Pin::new(body).poll_frame(context),
            Self::Dyn(boxed) => Pin::new(boxed).poll_frame(context),
            Self::Tee(tee) => Pin::new(tee).poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Inline(body) => body.is_end_stream(),
            Self::Dyn(boxed) => boxed.is_end_stream(),
            Self::Tee(tee) => tee.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Inline(body) => body.size_hint(),
            Self::Dyn(boxed) => boxed.size_hint(),
            Self::Tee(tee) => tee.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    fn from(body: BoxBody) -> Self {
        Self::Dyn(body)
    }
}

impl<T> From<TeeBody> for HttpBody<T> {
    fn from(body: TeeBody) -> Self {
        Self::Tee(body)
    }
}

impl From<RequestBody> for HttpBody<RequestBody> {
    #[inline]
    fn from(body: RequestBody) -> Self {
        Self::Inline(body)
    }
}
