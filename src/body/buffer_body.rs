use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::HttpBody;
use crate::BoxError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
const MAX_FRAME_LEN: usize = 8192; // 8KB

/// An in-memory byte buffer that is read in 8KB chunks.
///
#[must_use = "streams do nothing unless polled"]
pub struct BufferBody {
    /// The buffer containing the body data.
    ///
    buf: Bytes,

    /// The byte offset of the next data frame in `self.buf`.
    ///
    pos: usize,
}

impl BufferBody {
    #[inline]
    pub fn new(data: &[u8]) -> Self {
        Self {
            buf: Bytes::copy_from_slice(data),
            pos: 0,
        }
    }

    #[inline]
    pub fn from_raw(buf: Bytes) -> Self {
        Self { buf, pos: 0 }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    #[inline]
    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl Body for BufferBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        let remaining = this.remaining();

        // Check if the buffer has any data.
        if remaining == 0 {
            // The buffer is empty. Signal that the stream has ended.
            return Poll::Ready(None);
        }

        let lower = this.pos;
        let upper = lower + remaining.min(MAX_FRAME_LEN);
        let slice = this.buf.slice(lower..upper);

        this.pos = upper;

        // Split the buffer at the frame length. This will give us an owned
        // view of the frame at 0..frame_len and advance the buffer to the
        // offset of the next frame.
        Poll::Ready(Some(Ok(Frame::data(slice))))
    }

    fn is_end_stream(&self) -> bool {
        self.remaining() == 0
    }

    fn size_hint(&self) -> SizeHint {
        // Get the length of the buffer and attempt to cast it to a
        // `u64`. If the cast fails, `len` will be `None`.
        let len = self.len().try_into().ok();

        // If `len` is `None`, return a size hint with no bounds. Otherwise,
        // map the remaining length to a size hint with the exact size.
        len.map_or_else(SizeHint::new, SizeHint::with_exact)
    }
}

impl Debug for BufferBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ByteBuffer").finish()
    }
}

impl Default for BufferBody {
    fn default() -> Self {
        Self::from_raw(Bytes::new())
    }
}

impl From<Bytes> for BufferBody {
    fn from(buf: Bytes) -> Self {
        Self::from_raw(buf)
    }
}

impl From<Vec<u8>> for BufferBody {
    fn from(vec: Vec<u8>) -> Self {
        Self::from_raw(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for BufferBody {
    fn from(slice: &'static [u8]) -> Self {
        Self::new(slice)
    }
}

impl From<String> for BufferBody {
    fn from(string: String) -> Self {
        Self::from_raw(Bytes::from(string))
    }
}

impl From<&'static str> for BufferBody {
    fn from(slice: &'static str) -> Self {
        Self::new(slice.as_bytes())
    }
}

impl Default for HttpBody<BufferBody> {
    fn default() -> Self {
        HttpBody::Inline(Default::default())
    }
}

impl<T> From<T> for HttpBody<BufferBody>
where
    BufferBody: From<T>,
{
    fn from(body: T) -> Self {
        HttpBody::Inline(BufferBody::from(body))
    }
}
