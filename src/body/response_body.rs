use bytes::{Buf, Bytes};
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::DynError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
pub const MAX_FRAME_LEN: usize = 16384; // 16KB

/// A buffered `impl Body` that is read in `16KB` chunks.
///
pub struct ResponseBody {
    buf: Bytes,
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = DynError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let remaining = self.buf.remaining();

        if remaining == 0 {
            Poll::Ready(None)
        } else {
            Poll::Ready(Some(Ok(Frame::data(
                self.buf.split_to(remaining.min(MAX_FRAME_LEN)),
            ))))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.buf.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        match self.buf.len().try_into() {
            Ok(exact) => SizeHint::with_exact(exact),
            Err(error) => {
                // Placeholder for tracing...
                let _ = &error;
                SizeHint::new()
            }
        }
    }
}

impl Debug for ResponseBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ResponseBody").finish()
    }
}

impl Default for ResponseBody {
    #[inline]
    fn default() -> Self {
        Self { buf: Bytes::new() }
    }
}

impl From<Bytes> for ResponseBody {
    #[inline]
    fn from(data: Bytes) -> Self {
        Self { buf: data }
    }
}

impl From<String> for ResponseBody {
    #[inline]
    fn from(data: String) -> Self {
        Self::from(data.into_bytes())
    }
}

impl From<&'_ str> for ResponseBody {
    #[inline]
    fn from(data: &str) -> Self {
        Self::from(Bytes::copy_from_slice(data.as_bytes()))
    }
}

impl From<Vec<u8>> for ResponseBody {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self::from(Bytes::from(data))
    }
}

impl From<&'_ [u8]> for ResponseBody {
    #[inline]
    fn from(data: &'_ [u8]) -> Self {
        Self::from(Bytes::copy_from_slice(data))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http_body_util::BodyExt;

    use super::{ResponseBody, MAX_FRAME_LEN};

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
