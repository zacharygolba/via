use bytes::{Bytes, BytesMut};
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::http_body::HttpBody;
use crate::error::BoxError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
const MAX_FRAME_LEN: usize = 8192; // 8KB

/// An in-memory response body that is read in `8KB` chunks.
///
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    /// The buffer containing the body data.
    ///
    buf: BytesMut,
}

impl ResponseBody {
    #[inline]
    pub fn new(data: &[u8]) -> Self {
        Self::from_raw(Bytes::copy_from_slice(data))
    }

    #[inline]
    pub fn from_raw(bytes: Bytes) -> Self {
        Self {
            buf: BytesMut::from(bytes),
        }
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let len = this.buf.len();

        // Check if the buffer has any data.
        if len == 0 {
            // The buffer is empty. Signal that the stream has ended.
            Poll::Ready(None)
        } else {
            // Split the buffer at the frame length. This will give us an owned
            // view of the frame at 0..frame_len and advance the buffer to the
            // offset of the next frame.
            let next = this.buf.split_to(len.min(MAX_FRAME_LEN)).freeze();

            Poll::Ready(Some(Ok(Frame::data(next))))
        }
    }

    fn is_end_stream(&self) -> bool {
        self.buf.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        // Get the length of the buffer and attempt to cast it to a
        // `u64`. If the cast fails, return a size hint with no bounds.
        match self.buf.len().try_into() {
            Ok(exact) => SizeHint::with_exact(exact),
            Err(error) => {
                let _ = error; // Placeholder for tracing...
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
    fn default() -> Self {
        Self::from_raw(Bytes::new())
    }
}

impl From<Bytes> for ResponseBody {
    fn from(buf: Bytes) -> Self {
        Self::from_raw(buf)
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(vec: Vec<u8>) -> Self {
        Self::from_raw(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for ResponseBody {
    fn from(slice: &'static [u8]) -> Self {
        Self::new(slice)
    }
}

impl From<String> for ResponseBody {
    fn from(string: String) -> Self {
        Self::from_raw(Bytes::from(string))
    }
}

impl From<&'static str> for ResponseBody {
    fn from(slice: &'static str) -> Self {
        Self::new(slice.as_bytes())
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
