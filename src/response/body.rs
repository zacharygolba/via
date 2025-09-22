use bytes::{Buf, Bytes};
use http_body::{Body, Frame, SizeHint};
use http_body_util::Either;
use http_body_util::combinators::BoxBody;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::BoxError;

const DEFAULT_FRAME_LEN: usize = 8 * 1024; // 8KB
const ADAPTIVE_THRESHOLD: usize = 64 * 1024; // 64KB

/// A buffered `impl Body` that is written in `8KB..=16KB` chunks.
///
#[derive(Debug, Default)]
pub struct BufferBody(Bytes);

#[derive(Debug)]
pub struct ResponseBody(Either<BufferBody, BoxBody<Bytes, BoxError>>);

#[inline]
pub(super) fn adapt_frame_size(len: usize) -> usize {
    if len >= ADAPTIVE_THRESHOLD {
        DEFAULT_FRAME_LEN * 2
    } else {
        len.min(DEFAULT_FRAME_LEN)
    }
}

impl Body for BufferBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Self(buf) = self.get_mut();
        let remaining = buf.remaining();

        if remaining == 0 {
            Poll::Ready(None)
        } else {
            let len = adapt_frame_size(remaining);
            let data = buf.slice(..len);

            buf.advance(len);

            Poll::Ready(Some(Ok(Frame::data(data))))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        match self.0.len().try_into() {
            Ok(exact) => SizeHint::with_exact(exact),
            Err(_) => panic!("BufferBody::size_hint would overflow u64"),
        }
    }
}

impl From<Bytes> for BufferBody {
    #[inline]
    fn from(buf: Bytes) -> Self {
        Self(buf)
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

impl ResponseBody {
    /// Consume the response body and return a dynamically-dispatched
    /// [`BoxBody`] that is allocated on the heap.
    ///
    pub fn boxed(self) -> BoxBody<Bytes, BoxError> {
        match self {
            Self(Either::Left(inline)) => BoxBody::new(inline),
            Self(Either::Right(boxed)) => boxed,
        }
    }
}

impl ResponseBody {
    #[inline]
    pub(crate) fn map<U, F>(self, map: F) -> Self
    where
        F: FnOnce(ResponseBody) -> U,
        U: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self(Either::Right(BoxBody::new(map(self))))
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Self(body) = self.get_mut();
        Pin::new(body).poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }
}

impl From<BoxBody<Bytes, BoxError>> for ResponseBody {
    #[inline]
    fn from(body: BoxBody<Bytes, BoxError>) -> Self {
        ResponseBody(Either::Right(body))
    }
}

impl<T> From<T> for ResponseBody
where
    BufferBody: From<T>,
{
    #[inline]
    fn from(body: T) -> Self {
        ResponseBody(Either::Left(body.into()))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body::Body;
    use http_body_util::BodyExt;

    use super::{BufferBody, DEFAULT_FRAME_LEN};

    #[tokio::test]
    async fn test_is_end_stream() {
        assert!(
            BufferBody::default().is_end_stream(),
            "is_end_stream should be true for an empty response"
        );

        let mut body =
            BufferBody::from(format!("Hello,{}world", " ".repeat(DEFAULT_FRAME_LEN - 6)));

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
            format!("hello{}", " ".repeat(DEFAULT_FRAME_LEN - 5)),
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
