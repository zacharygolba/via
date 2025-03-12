use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use super::request_body::RequestBody;
use super::response_body::ResponseBody;
use super::tee_body::TeeBody;
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
    Original(T),

    /// A type erased, dynamically dispatched [`Body`].
    ///
    Boxed(BoxBody),

    /// A boxed body that writes each data frame into a dyn
    /// [`AsyncWrite`](tokio::io::AsyncWrite).
    ///
    Tee(TeeBody),
}

impl HttpBody<ResponseBody> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T> HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + Unpin + 'static,
{
    #[inline]
    pub fn boxed(self) -> Self {
        Self::Boxed(BoxBody::new(self))
    }

    #[inline]
    pub fn tee(self, io: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self::Tee(TeeBody::new(BoxBody::new(self), io))
    }
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
            Self::Original(body) => Pin::new(body).poll_frame(context),
            Self::Boxed(body) => Pin::new(body).poll_frame(context),
            Self::Tee(body) => Pin::new(body).poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Original(body) => body.is_end_stream(),
            Self::Boxed(body) => body.is_end_stream(),
            Self::Tee(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Original(body) => body.size_hint(),
            Self::Boxed(body) => body.size_hint(),
            Self::Tee(body) => body.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    #[inline]
    fn from(body: BoxBody) -> Self {
        Self::Boxed(body)
    }
}

impl<T> From<TeeBody> for HttpBody<T> {
    #[inline]
    fn from(body: TeeBody) -> Self {
        Self::Tee(body)
    }
}

impl From<RequestBody> for HttpBody<RequestBody> {
    #[inline]
    fn from(body: RequestBody) -> Self {
        Self::Original(body)
    }
}

impl Default for HttpBody<ResponseBody> {
    #[inline]
    fn default() -> Self {
        Self::Original(Default::default())
    }
}

impl<T> From<T> for HttpBody<ResponseBody>
where
    ResponseBody: From<T>,
{
    fn from(body: T) -> Self {
        Self::Original(ResponseBody::from(body))
    }
}
