use bytes::Bytes;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http_body::{Body, Frame, SizeHint};
use http_body_util::Either;
use http_body_util::combinators::BoxBody;
use serde::Serialize;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::builder::{Finalize, ResponseBuilder};
use super::response::Response;
use crate::error::{BoxError, Error};

pub(super) const MAX_FRAME_SIZE: usize = 16 * 1024; // 16KB

/// Serialize the contained type as an untagged JSON response.
///
/// # Example
/// ```
/// use serde::Serialize;
/// use via::response::{Finalize, Json, Response};
///
/// #[derive(Serialize)]
/// struct Cat {
///     name: String,
/// }
///
/// let ciro = Cat {
///     name: "Ciro".to_owned(),
/// };
///
/// let tagged = Response::build().json(&ciro).unwrap();
/// // => { "data": { "name": "Ciro" } }
///
/// let untagged = Json(&ciro).into_response().unwrap();
/// // => { "name": "Ciro" }
/// ```
///
#[derive(Debug)]
pub struct Json<'a, T>(pub &'a T);

/// A buffered `impl Body` that is written in `16 KB` chunks.
///
#[derive(Default)]
pub struct BufferBody {
    buf: Bytes,
}

#[derive(Debug)]
pub struct ResponseBody {
    kind: Either<BufferBody, BoxBody<Bytes, BoxError>>,
}

impl Body for BufferBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Self { buf } = self.get_mut();
        let len = buf.len().min(MAX_FRAME_SIZE);

        Poll::Ready((len > 0).then(|| Ok(Frame::data(buf.split_to(len)))))
    }

    fn is_end_stream(&self) -> bool {
        self.buf.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        self.buf
            .len()
            .try_into()
            .map(SizeHint::with_exact)
            .unwrap_or_default()
    }
}

impl Debug for BufferBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("BufferBody").finish()
    }
}

impl From<Bytes> for BufferBody {
    #[inline]
    fn from(buf: Bytes) -> Self {
        Self { buf }
    }
}

impl From<String> for BufferBody {
    #[inline]
    fn from(data: String) -> Self {
        Self::from(data.into_bytes())
    }
}

impl From<&'_ str> for BufferBody {
    #[inline]
    fn from(data: &str) -> Self {
        Self::from(data.as_bytes())
    }
}

impl From<Vec<u8>> for BufferBody {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self::from(Bytes::from(data))
    }
}

impl From<&'_ [u8]> for BufferBody {
    #[inline]
    fn from(slice: &'_ [u8]) -> Self {
        Self::from(Bytes::copy_from_slice(slice))
    }
}

impl<'a, T> Finalize for Json<'a, T>
where
    T: Serialize,
{
    #[inline]
    fn finalize(self, response: ResponseBuilder) -> Result<Response, Error> {
        let Self(json) = self;
        let payload = serde_json::to_vec(json)?;

        response
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header(CONTENT_LENGTH, payload.len())
            .body(payload)
    }
}

impl ResponseBody {
    pub(crate) fn map<U, F>(self, map: F) -> Self
    where
        F: FnOnce(ResponseBody) -> U,
        U: http_body::Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self {
            kind: Either::Right(BoxBody::new(map(self))),
        }
    }

    /// Consume the response body and return a dynamically-dispatched
    /// [`BoxBody`] that is allocated on the heap.
    ///
    pub fn boxed(self) -> BoxBody<Bytes, BoxError> {
        match self.kind {
            Either::Left(inline) => BoxBody::new(inline),
            Either::Right(boxed) => boxed,
        }
    }
}

impl http_body::Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Self { kind } = self.get_mut();

        match kind {
            Either::Left(inline) => Pin::new(inline).poll_frame(context),
            Either::Right(boxed) => Pin::new(boxed).poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.kind.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.kind.size_hint()
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self {
            kind: Either::Left(Default::default()),
        }
    }
}

impl From<BoxBody<Bytes, BoxError>> for ResponseBody {
    #[inline]
    fn from(body: BoxBody<Bytes, BoxError>) -> Self {
        Self {
            kind: Either::Right(body),
        }
    }
}

impl<T> From<T> for ResponseBody
where
    BufferBody: From<T>,
{
    #[inline]
    fn from(body: T) -> Self {
        Self {
            kind: Either::Left(body.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body::Body;
    use http_body_util::BodyExt;

    use super::{BufferBody, MAX_FRAME_SIZE};

    #[tokio::test]
    async fn test_is_end_stream() {
        assert!(
            BufferBody::default().is_end_stream(),
            "is_end_stream should be true for an empty response"
        );

        let mut body = BufferBody::from(format!("Hello,{}world", " ".repeat(MAX_FRAME_SIZE - 6)));

        assert!(
            !body.is_end_stream(),
            "is_end_stream should be false when there is a remaining data frame."
        );

        while body.frame().await.is_some() {}

        assert!(
            body.is_end_stream(),
            "is_end_stream should be true after each frame is polled."
        );
    }

    #[test]
    fn test_size_hint() {
        let hint = Body::size_hint(&BufferBody::from("Hello, world!"));
        assert_eq!(hint.exact(), Some("Hello, world!".len() as u64));
    }

    #[tokio::test]
    async fn test_poll_frame_empty() {
        assert!(BufferBody::default().frame().await.is_none());
    }

    #[tokio::test]
    async fn test_poll_frame_one() {
        let mut body = BufferBody::from("Hello, world!");
        let hello_world = body.frame().await.unwrap().unwrap().into_data().unwrap();

        assert_eq!(hello_world, Bytes::copy_from_slice(b"Hello, world!"));
        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn test_poll_frame() {
        let frames = [
            format!("hello{}", " ".repeat(MAX_FRAME_SIZE - 5)),
            "world".to_owned(),
        ];

        let mut body = BufferBody::from(frames.concat());

        for part in &frames {
            let next = body.frame().await.unwrap().unwrap().into_data().unwrap();
            assert_eq!(next, Bytes::copy_from_slice(part.as_bytes()));
        }

        assert!(body.frame().await.is_none());
    }
}
