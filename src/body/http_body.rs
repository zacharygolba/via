use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use super::body_reader::{BodyData, BodyReader};
use super::body_stream::BodyStream;
use super::request_body::RequestBody;
use super::response_body::ResponseBody;
use super::tee_body::TeeBody;
use crate::error::{DynError, Error};

/// A type erased, dynamically dispatched [`Body`].
///
pub type BoxBody = http_body_util::combinators::BoxBody<Bytes, DynError>;

/// The body of a request or response.
///
#[derive(Debug)]
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
    Tee(TeeBody<T>),
}

impl<T> HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + Unpin + 'static,
{
    pub fn boxed(self) -> Self {
        Self::Boxed(match self {
            Self::Original(body) => BoxBody::new(body),
            Self::Boxed(body) => body,
            Self::Tee(body) => BoxBody::new(body),
        })
    }

    pub fn tee(self, io: impl AsyncWrite + Send + Sync + 'static) -> Self {
        match self.try_into_inner() {
            Ok(original) => Self::Tee(TeeBody::new(original, io)),
            Err(boxed) => {
                // Placeholder for tracing...
                // warn!("tip: call tee() before boxed() to avoid an extra allocation.");
                Self::Boxed(BoxBody::new(TeeBody::new(boxed, io)))
            }
        }
    }

    pub fn try_into_inner(self) -> Result<T, BoxBody> {
        match self {
            Self::Original(original) => Ok(original),
            Self::Boxed(boxed) => Err(boxed),
            Self::Tee(tee) => Ok(tee.cap()),
        }
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
        match &self {
            Self::Original(body) => body.is_end_stream(),
            Self::Boxed(body) => body.is_end_stream(),
            Self::Tee(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self {
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

impl<T> From<TeeBody<T>> for HttpBody<T> {
    #[inline]
    fn from(body: TeeBody<T>) -> Self {
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
    #[inline]
    fn from(body: T) -> Self {
        Self::Original(body.into())
    }
}
