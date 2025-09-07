use bytes::{Buf, Bytes};
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::BoxError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
pub const MIN_FRAME_LEN: usize = 8 * 1024; // 8KB
pub const MAX_FRAME_LEN: usize = 16 * 1024; // 16KB

const ADAPTIVE_THRESHOLD: usize = 64 * 1024; // 64KB

/// A buffered `impl Body` that is read in `8KB` chunks.
///
#[derive(Debug, Default)]
pub struct BufferBody {
    max: usize,
    data: Bytes,
}

fn adapt_frame_size(len: usize) -> usize {
    if len >= ADAPTIVE_THRESHOLD {
        MAX_FRAME_LEN
    } else {
        MIN_FRAME_LEN
    }
}

impl Body for BufferBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let remaining = self.data.remaining();

        if remaining == 0 {
            Poll::Ready(None)
        } else {
            let len = remaining.min(self.max);
            let frame = self.data.split_to(len);

            Poll::Ready(Some(Ok(Frame::data(frame))))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        match self.data.len().try_into() {
            Ok(exact) => SizeHint::with_exact(exact),
            Err(_) => panic!("BufferBody::size_hint would overflow u64"),
        }
    }
}

impl From<Bytes> for BufferBody {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self {
            max: adapt_frame_size(data.len()),
            data,
        }
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
        Self {
            max: adapt_frame_size(data.len()),
            data: Bytes::from(data),
        }
    }
}

impl From<&'_ [u8]> for BufferBody {
    #[inline]
    fn from(slice: &'_ [u8]) -> Self {
        Self {
            max: adapt_frame_size(slice.len()),
            data: Bytes::copy_from_slice(slice),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body::Body;
    use http_body_util::BodyExt;

    use super::{BufferBody, MIN_FRAME_LEN};

    #[tokio::test]
    async fn test_is_end_stream() {
        assert!(
            BufferBody::default().is_end_stream(),
            "is_end_stream should be true for an empty response"
        );

        let mut body = BufferBody::from(format!("Hello,{}world", " ".repeat(MIN_FRAME_LEN - 6)));

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
            format!("hello{}", " ".repeat(MIN_FRAME_LEN - 5)),
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
