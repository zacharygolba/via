use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// The maximum amount of data that can be read from a buffered body per frame.
const MAX_FRAME_LEN: usize = 16384; // 16KB

/// An optimized body that is used when the entire body is already in memory.
#[must_use = "streams do nothing unless polled"]
pub struct Buffer {
    /// The buffer containing the body data.
    buf: Bytes,
}

impl Buffer {
    pub fn new(buf: Bytes) -> Self {
        Self { buf }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl Buffer {
    pub fn buf_mut(&mut self) -> &mut Bytes {
        &mut self.buf
    }
}

impl Body for Buffer {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // Get a mutable reference to `self.buf`.
        let buf = self.buf_mut();

        // Get the number of bytes to read from `buffer` for the current frame.
        // We read a maximum of 16KB per frame from `buffer`.
        let len = buf.len().min(MAX_FRAME_LEN);

        // Check if the buffer has any data.
        if len == 0 {
            // The buffer is empty. Signal that the stream has ended.
            return Poll::Ready(None);
        }

        // Split the buffer at the frame length. This will give us an owned
        // view of the frame at 0..frame_len and advance the buffer to the
        // offset of the next frame.
        let data = buf.split_to(len);
        // Wrap the bytes we copied from buffer in a data frame.
        let frame = Frame::data(data);

        // Return the data frame to the caller.
        Poll::Ready(Some(Ok(frame)))
    }

    fn is_end_stream(&self) -> bool {
        self.is_empty()
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

impl Debug for Buffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let len = self.buf.len();
        f.debug_struct("Buffer").field("len", &len).finish()
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new(Bytes::new())
    }
}

impl From<Bytes> for Buffer {
    fn from(bytes: Bytes) -> Self {
        Self::new(Bytes::copy_from_slice(&bytes))
    }
}

impl From<Vec<u8>> for Buffer {
    fn from(vec: Vec<u8>) -> Self {
        Self::new(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for Buffer {
    fn from(slice: &'static [u8]) -> Self {
        Self::new(Bytes::from_static(slice))
    }
}

impl From<String> for Buffer {
    fn from(string: String) -> Self {
        let vec = string.into_bytes();
        Self::new(Bytes::from(vec))
    }
}

impl From<&'static str> for Buffer {
    fn from(slice: &'static str) -> Self {
        let slice = slice.as_bytes();
        Self::new(Bytes::from_static(slice))
    }
}
