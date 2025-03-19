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
    Inline(T),

    /// A type erased, dynamically dispatched [`Body`].
    ///
    Dyn(BoxBody),
}

impl<T> HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + Unpin + 'static,
{
    pub fn boxed(self) -> BoxBody {
        match self {
            Self::Inline(body) => BoxBody::new(body),
            Self::Dyn(body) => body,
        }
    }

    pub fn tee<U>(self, dest: U) -> Self
    where
        U: AsyncWrite + Send + Sync + Unpin + 'static,
    {
        Self::Dyn(match self {
            Self::Inline(src) => BoxBody::new(TeeBody::new(src, dest)),
            Self::Dyn(src) => BoxBody::new(TeeBody::new(src, dest)),
        })
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
            Self::Inline(body) => Pin::new(body).poll_frame(context),
            Self::Dyn(body) => Pin::new(body).poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self {
            Self::Inline(body) => body.is_end_stream(),
            Self::Dyn(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self {
            Self::Inline(body) => body.size_hint(),
            Self::Dyn(body) => body.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    #[inline]
    fn from(body: BoxBody) -> Self {
        Self::Dyn(body)
    }
}

impl From<RequestBody> for HttpBody<RequestBody> {
    #[inline]
    fn from(body: RequestBody) -> Self {
        Self::Inline(body)
    }
}

impl Default for HttpBody<ResponseBody> {
    #[inline]
    fn default() -> Self {
        Self::Inline(Default::default())
    }
}

impl<T> From<T> for HttpBody<ResponseBody>
where
    ResponseBody: From<T>,
{
    #[inline]
    fn from(body: T) -> Self {
        Self::Inline(body.into())
    }
}
