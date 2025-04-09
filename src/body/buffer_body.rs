use bytes::{Buf, Bytes};
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::DynError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
pub const MAX_FRAME_LEN: usize = 8192; // 8KB

/// A buffered `impl Body` that is read in `8KB` chunks.
///
#[derive(Debug, Default)]
pub struct BufferBody {
    data: Bytes,
}

impl Body for BufferBody {
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let remaining = self.data.remaining();

        if remaining == 0 {
            Poll::Ready(None)
        } else {
            Poll::Ready(Some(Ok(Frame::data(
                self.data.split_to(remaining.min(MAX_FRAME_LEN)),
            ))))
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
        Self { data }
    }
}

impl From<String> for BufferBody {
    #[inline]
    fn from(data: String) -> Self {
        Self {
            data: Bytes::from(data),
        }
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
            data: Bytes::from(data),
        }
    }
}

impl From<&'_ [u8]> for BufferBody {
    #[inline]
    fn from(slice: &'_ [u8]) -> Self {
        Self {
            data: Bytes::copy_from_slice(slice),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body::Body;
    use http_body_util::BodyExt;

    use super::{BufferBody, MAX_FRAME_LEN};

    #[tokio::test]
    async fn test_is_end_stream() {
        assert!(
            BufferBody::default().is_end_stream(),
            "is_end_stream should be true for an empty response"
        );

        let mut body = BufferBody::from(format!("Hello,{}world", " ".repeat(MAX_FRAME_LEN - 6)));

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
        let single_frame = Body::size_hint(&BufferBody::from("Hello, world!"));
        let many_frames = Body::size_hint(&BufferBody::from(format!(
            "Hello,{}world",
            " ".repeat(MAX_FRAME_LEN - 6)
        )));

        assert_eq!(single_frame.exact(), Some("Hello, world!".len() as u64));
        assert_eq!(many_frames.exact(), Some("Hello, world!".len() as u64));
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
            format!("hello{}", " ".repeat(MAX_FRAME_LEN - 5)),
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
