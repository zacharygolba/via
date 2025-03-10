use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};

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

    ///
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
    T: Body<Data = Bytes, Error = DynError> + Debug + Send + Sync + 'static,
{
    /// Returns a BoxBody from the `impl Body` contained in self.
    ///
    /// If `self` is `HttpBody::Boxed`, no additional allocations are made.
    ///
    pub fn boxed(self) -> BoxBody {
        match self {
            HttpBody::Original(body) => BoxBody::new(body),
            HttpBody::Boxed(body) => body,
            HttpBody::Tee(body) => body.cap(),
        }
    }

    /// Returns the original [RequestBody] or [ResponseBody]
    ///
    /// ## Panics
    ///
    /// If `self` is not `HttpBody::Original`.
    ///
    pub fn into_inner(self) -> T {
        self.try_into_inner()
            .expect("expected self to be HttpBody::Original")
    }

    /// Unwrap the [`Body`] contained in self, if self is `HttpBody::Original`.
    ///
    /// If self is not `HttpBody::Original`, ownership of self is transferred
    /// back to the caller.
    ///
    pub fn try_into_inner(self) -> Result<T, Self> {
        match self {
            body @ (HttpBody::Boxed(_) | HttpBody::Tee(_)) => Err(body),
            HttpBody::Original(body) => Ok(body),
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
        HttpBody::Original(Default::default())
    }
}

impl<T> From<T> for HttpBody<ResponseBody>
where
    ResponseBody: From<T>,
{
    fn from(body: T) -> Self {
        HttpBody::Original(ResponseBody::from(body))
    }
}
