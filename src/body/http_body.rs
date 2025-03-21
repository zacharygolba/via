use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::body_reader::{BodyData, BodyReader};
use super::body_stream::BodyStream;
use super::request_body::RequestBody;
use super::response_body::ResponseBody;
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
    Initial(T),

    /// A type erased, dynamically dispatched [`Body`].
    ///
    Boxed(BoxBody),
}

enum HttpBodyProjection<'a, T> {
    Initial(Pin<&'a mut T>),
    Boxed(Pin<&'a mut BoxBody>),
}

impl<T> HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError> + Send + Sync + 'static,
{
    pub fn into_box_body(self) -> BoxBody {
        match self {
            Self::Initial(body) => BoxBody::new(body),
            Self::Boxed(body) => body,
        }
    }

    pub fn try_into_box_body(self) -> Result<BoxBody, T> {
        match self {
            Self::Initial(body) => Err(body),
            Self::Boxed(body) => Ok(body),
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
    fn project(self: Pin<&mut Self>) -> HttpBodyProjection<T> {
        unsafe {
            match self.get_unchecked_mut() {
                Self::Initial(body) => HttpBodyProjection::Initial(Pin::new_unchecked(body)),
                Self::Boxed(body) => HttpBodyProjection::Boxed(Pin::new_unchecked(body)),
            }
        }
    }
}

impl<T> Body for HttpBody<T>
where
    T: Body<Data = Bytes, Error = DynError>,
{
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            HttpBodyProjection::Initial(body) => body.poll_frame(context),
            HttpBodyProjection::Boxed(body) => body.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Initial(body) => body.is_end_stream(),
            Self::Boxed(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Initial(body) => body.size_hint(),
            Self::Boxed(body) => body.size_hint(),
        }
    }
}

impl<T> From<BoxBody> for HttpBody<T> {
    #[inline]
    fn from(body: BoxBody) -> Self {
        Self::Boxed(body)
    }
}

impl Default for HttpBody<ResponseBody> {
    #[inline]
    fn default() -> Self {
        Self::Initial(Default::default())
    }
}

impl<T> From<T> for HttpBody<ResponseBody>
where
    ResponseBody: From<T>,
{
    #[inline]
    fn from(body: T) -> Self {
        Self::Initial(body.into())
    }
}
