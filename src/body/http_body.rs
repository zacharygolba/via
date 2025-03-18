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
pub struct HttpBody<T> {
    kind: BodyKind<T>,
}

enum BodyProjection<'a, T> {
    Original(Pin<&'a mut T>),
    Boxed(Pin<&'a mut BoxBody>),
    Tee(Pin<&'a mut TeeBody>),
}

#[derive(Debug)]
enum BodyKind<T> {
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

impl<T> BodyKind<T>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + 'static,
{
    #[inline(always)]
    fn boxed(self) -> BoxBody {
        match self {
            Self::Original(body) => BoxBody::new(body),
            Self::Boxed(body) => body,
            Self::Tee(body) => BoxBody::new(body),
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

impl<T> HttpBody<T> {
    #[inline(always)]
    pub(crate) fn new(body: T) -> Self {
        Self {
            kind: BodyKind::Original(body),
        }
    }

    pub fn try_into_inner(self) -> Result<T, Self> {
        match self.kind {
            BodyKind::Boxed(_) | BodyKind::Tee(_) => Err(self),
            BodyKind::Original(body) => Ok(body),
        }
    }
}

impl<T> HttpBody<T> {
    fn project(self: Pin<&mut Self>) -> BodyProjection<T> {
        unsafe {
            match &mut self.get_unchecked_mut().kind {
                BodyKind::Original(body) => BodyProjection::Original(Pin::new_unchecked(body)),
                BodyKind::Boxed(body) => BodyProjection::Boxed(Pin::new_unchecked(body)),
                BodyKind::Tee(body) => BodyProjection::Tee(Pin::new_unchecked(body)),
            }
        }
    }
}

impl<T> HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + 'static,
{
    #[inline]
    pub fn boxed(self) -> Self {
        Self {
            kind: BodyKind::Boxed(self.kind.boxed()),
        }
    }

    #[inline]
    pub fn tee(self, io: impl AsyncWrite + Send + Sync + 'static) -> Self {
        Self {
            kind: BodyKind::Tee(TeeBody::new(self.kind.boxed(), io)),
        }
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
        match self.project() {
            BodyProjection::Original(body) => body.poll_frame(context),
            BodyProjection::Boxed(body) => body.poll_frame(context),
            BodyProjection::Tee(body) => body.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            BodyKind::Original(body) => body.is_end_stream(),
            BodyKind::Boxed(body) => body.is_end_stream(),
            BodyKind::Tee(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            BodyKind::Original(body) => body.size_hint(),
            BodyKind::Boxed(body) => body.size_hint(),
            BodyKind::Tee(body) => body.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    #[inline]
    fn from(body: BoxBody) -> Self {
        Self {
            kind: BodyKind::Boxed(body),
        }
    }
}

impl<T> From<TeeBody> for HttpBody<T> {
    #[inline]
    fn from(body: TeeBody) -> Self {
        Self {
            kind: BodyKind::Tee(body),
        }
    }
}

impl From<RequestBody> for HttpBody<RequestBody> {
    #[inline]
    fn from(body: RequestBody) -> Self {
        Self {
            kind: BodyKind::Original(body),
        }
    }
}

impl Default for HttpBody<ResponseBody> {
    #[inline]
    fn default() -> Self {
        Self {
            kind: BodyKind::Original(Default::default()),
        }
    }
}

impl<T> From<T> for HttpBody<ResponseBody>
where
    ResponseBody: From<T>,
{
    #[inline]
    fn from(body: T) -> Self {
        Self {
            kind: BodyKind::Original(ResponseBody::from(body)),
        }
    }
}
