use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use http_body_util::Either;
use http_body_util::combinators::BoxBody;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::BoxError;

const STANDARD_FRAME_LEN: usize = 8 * 1024; // 8KB
const EXTENDED_FRAME_LEN: usize = STANDARD_FRAME_LEN * 2; // 16KB
const ADAPTIVE_THRESHOLD: usize = 64 * 1024; // 64KB

/// A buffered `impl Body` that is written in `8KB..=16KB` chunks.
///
#[derive(Default)]
pub struct BufferBody {
    buf: Bytes,
}

#[derive(Debug)]
pub struct ResponseBody {
    kind: Either<BufferBody, BoxBody<Bytes, BoxError>>,
}

#[inline]
pub(super) fn adapt_frame_size(len: usize) -> usize {
    len.min(if len >= ADAPTIVE_THRESHOLD {
        EXTENDED_FRAME_LEN
    } else {
        STANDARD_FRAME_LEN
    })
}

impl Body for BufferBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Self { buf } = self.get_mut();
        let remaining = buf.len();

        if remaining == 0 {
            Poll::Ready(None)
        } else {
            let len = adapt_frame_size(remaining);
            Poll::Ready(Some(Ok(Frame::data(buf.split_to(len)))))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.buf.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        self.buf
            .len()
            .try_into()
            .map(SizeHint::with_exact)
            .expect("BufferBody::size_hint would overflow u64")
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

impl ResponseBody {
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

impl ResponseBody {
    pub(crate) fn map<U, F>(self, map: F) -> Self
    where
        F: FnOnce(ResponseBody) -> U,
        U: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self {
            kind: Either::Right(BoxBody::new(map(self))),
        }
    }
}

impl Body for ResponseBody {
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

    use super::{BufferBody, STANDARD_FRAME_LEN};

    #[tokio::test]
    async fn test_is_end_stream() {
        assert!(
            BufferBody::default().is_end_stream(),
            "is_end_stream should be true for an empty response"
        );

        let mut body =
            BufferBody::from(format!("Hello,{}world", " ".repeat(STANDARD_FRAME_LEN - 6)));

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
            format!("hello{}", " ".repeat(STANDARD_FRAME_LEN - 5)),
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
