use bytes::Bytes;
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
pub struct ResponseBody {
    offset: usize,
    chunks: Vec<Bytes>,
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.chunks.get(self.offset).cloned() {
            None => Poll::Ready(None),
            Some(data) => {
                self.offset += 1;
                Poll::Ready(Some(Ok(Frame::data(data))))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.offset >= self.chunks.len()
    }

    fn size_hint(&self) -> SizeHint {
        // The following type casts are safe because:
        //
        //   - MAX_FRAME_LEN is a constant
        //   - rest.len() is never > isize::MAX
        //   - last.len() is never > MAX_FRAME_LEN
        //
        self.chunks
            .iter()
            .try_fold(0u64, |n, chunk| n.checked_add(chunk.len() as u64))
            .map_or_else(SizeHint::new, SizeHint::with_exact)
    }
}

impl From<Bytes> for ResponseBody {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self::from(data.as_ref())
    }
}

impl From<String> for ResponseBody {
    #[inline]
    fn from(data: String) -> Self {
        Self::from(data.as_bytes())
    }
}

impl From<&'_ str> for ResponseBody {
    #[inline]
    fn from(data: &str) -> Self {
        Self::from(data.as_bytes())
    }
}

impl From<Vec<u8>> for ResponseBody {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self::from(data.as_slice())
    }
}

impl From<&'_ [u8]> for ResponseBody {
    #[inline]
    fn from(slice: &'_ [u8]) -> Self {
        let mut chunks = Vec::with_capacity(slice.len().div_ceil(MAX_FRAME_LEN));

        for chunk in slice.chunks(MAX_FRAME_LEN) {
            chunks.push(Bytes::copy_from_slice(chunk));
        }

        Self { offset: 0, chunks }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body::Body;
    use http_body_util::BodyExt;

    use super::{ResponseBody, MAX_FRAME_LEN};

    impl ResponseBody {
        fn new(chunks: Vec<Bytes>) -> Self {
            Self { offset: 0, chunks }
        }
    }

    #[test]
    fn test_response_body_from_bytes() {
        let body = ResponseBody::from(Bytes::copy_from_slice(&vec![b' '; MAX_FRAME_LEN * 10]));

        assert_eq!(
            body.chunks.len(),
            10,
            "the byte buffer is split into chunks no larger than MAX_FRAME_LEN"
        );

        assert_eq!(
            body.chunks.len(),
            body.chunks.capacity(),
            "the capacity of the vec that stores each chunk is also it's length"
        );
    }

    #[tokio::test]
    async fn test_is_end_stream() {
        assert!(
            ResponseBody::default().is_end_stream(),
            "is_end_stream should be true for an empty response"
        );

        let mut body = ResponseBody::new(vec![
            Bytes::copy_from_slice(b"Hello"),
            Bytes::copy_from_slice(b", world!"),
        ]);

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
        let single_frame = Body::size_hint(&ResponseBody::new(vec![Bytes::copy_from_slice(
            b"Hello, world!",
        )]));

        let many_frames = Body::size_hint(&ResponseBody::new(vec![
            Bytes::copy_from_slice(b"Hello"),
            Bytes::copy_from_slice(b", world!"),
        ]));

        assert_eq!(single_frame.exact(), Some("Hello, world!".len() as u64));
        assert_eq!(many_frames.exact(), Some("Hello, world!".len() as u64));
    }

    #[tokio::test]
    async fn test_poll_frame_empty() {
        let mut body = ResponseBody::from("");
        assert!(body.frame().await.is_none());
    }

    #[tokio::test]
    async fn test_poll_frame_one() {
        let mut body = ResponseBody::from("Hello, world!");
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

        let mut body = ResponseBody::from(frames.concat());

        for part in &frames {
            let next = body.frame().await.unwrap().unwrap().into_data().unwrap();
            assert_eq!(next, Bytes::copy_from_slice(part.as_bytes()));
        }

        assert!(body.frame().await.is_none());
    }
}
