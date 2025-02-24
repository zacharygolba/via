use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::http_body::HttpBody;
use crate::error::BoxError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
const MAX_FRAME_LEN: usize = 8192; // 8KB

/// A buffered `impl Body` that is read in `8KB` chunks.
///
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    data: Bytes,
    remaining: usize,
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.remaining == 0 {
            // All bytes have been read out of self. 🙌
            return Poll::Ready(None);
        }

        // The start offset of the next frame.
        let from = self.data.len() - self.remaining;

        // The byte length of the next frame.
        let len = self.remaining.min(MAX_FRAME_LEN);

        // The end offset of the next frame.
        let to = from + len;

        // Decrement remaining by len.
        self.remaining -= len;

        // Increment the ref-count of the underlying byte buffer and return an
        // owned slice containing the bytes at from..to.
        Poll::Ready(Some(Ok(Frame::data(self.data.slice(from..to)))))
    }

    fn is_end_stream(&self) -> bool {
        self.remaining == 0
    }

    fn size_hint(&self) -> SizeHint {
        match self.data.len().try_into() {
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
        Self {
            data: Bytes::new(),
            remaining: 0,
        }
    }
}

impl From<Bytes> for ResponseBody {
    #[inline]
    fn from(data: Bytes) -> Self {
        let remaining = data.len();
        Self { data, remaining }
    }
}

impl From<String> for ResponseBody {
    #[inline]
    fn from(data: String) -> Self {
        Self::from(Bytes::copy_from_slice(data.as_bytes()))
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
        Self::from(Bytes::copy_from_slice(&data))
    }
}

impl From<&'_ [u8]> for ResponseBody {
    #[inline]
    fn from(data: &'_ [u8]) -> Self {
        Self::from(Bytes::copy_from_slice(data))
    }
}

impl HttpBody<ResponseBody> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
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
