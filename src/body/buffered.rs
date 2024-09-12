use bytes::{Bytes, BytesMut};
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// The maximum amount of data that can be read from a buffered body per frame.
const MAX_FRAME_LEN: usize = 16384; // 16KB

/// An optimized body that is used when the entire body is already in memory.
#[must_use = "streams do nothing unless polled"]
pub struct BufferedBody {
    /// The buffer containing the body data.
    buf: BytesMut,

    /// A marker type indicating that the `BufferedBody` is pinned in memory.
    _pin: PhantomPinned,
}

impl BufferedBody {
    pub fn new(data: &[u8]) -> Self {
        let unique = Bytes::copy_from_slice(data);

        Self {
            buf: BytesMut::from(unique),
            _pin: PhantomPinned,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl BufferedBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut BytesMut> {
        //
        // Safety:
        //
        // No data is moved out of self or the `buf` field in the APIs that
        // we use in the implementation of Body::poll_frame for BufferedBody.
        // Furthermore, the bytes crate is frozen to a version that we know does
        // not move the buffer in any of it's APIs.
        //
        // It is integral to the safety of this pin projection that future
        // upgrades of the bytes crate are audited to ensure that the internal
        // buffer of BytesMut is not moved in any of the APIs that are called in
        // the implementation of Body::poll_frame.
        //
        unsafe { Pin::map_unchecked_mut(self, |this| &mut this.buf) }
    }
}

impl Body for BufferedBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // Get a mutable reference to the `buf` field.
        let mut buf = self.project();

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
        let data = buf.split_to(len).freeze();
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

impl Debug for BufferedBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferedBody").finish()
    }
}

impl Default for BufferedBody {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl From<Bytes> for BufferedBody {
    fn from(bytes: Bytes) -> Self {
        Self::new(&bytes)
    }
}

impl From<Vec<u8>> for BufferedBody {
    fn from(vec: Vec<u8>) -> Self {
        Self::new(vec.as_slice())
    }
}

impl From<&'static [u8]> for BufferedBody {
    fn from(slice: &'static [u8]) -> Self {
        Self::new(slice)
    }
}

impl From<String> for BufferedBody {
    fn from(string: String) -> Self {
        Self::new(string.as_bytes())
    }
}

impl From<&'static str> for BufferedBody {
    fn from(slice: &'static str) -> Self {
        Self::new(slice.as_bytes())
    }
}
