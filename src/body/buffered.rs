use bytes::{Bytes, BytesMut};
use hyper::body::{Body, Frame, SizeHint};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// The maximum amount of data that can be read from a buffered body per frame.
const MAX_FRAME_LEN: usize = 16384; // 16KB

/// An optimized body that is used when the entire body is already in memory.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Buffered {
    /// The buffer containing the body data.
    data: Box<BytesMut>,

    /// A marker indicating that the type is pinned to prevent moving the buffer.
    _pin: PhantomPinned,
}

impl Buffered {
    pub fn new(data: BytesMut) -> Self {
        Self {
            data: Box::new(data),
            _pin: PhantomPinned,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Buffered {
    /// Returns a pinned mutable reference to the `buffer` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut BytesMut> {
        unsafe {
            //
            // Safety:
            //
            // Data is never moved out of self or the buffer stored at `self.data`.
            // We use `BytesMut::split_to` in combination with `BytesMut::freeze`
            // to ensure that data is copied out of the buffer and the cursor is
            // advanced to the offset of the next frame.
            //

            // Get a mutable reference to `self`.
            let this = self.get_unchecked_mut();

            // Get a mutable reference to the buffer at `self.data`.
            let ptr = &mut *this.data;

            // Return a pinned mutable reference to the buffer at `self.data`.
            Pin::new_unchecked(ptr)
        }
    }
}

impl Body for Buffered {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // Get a pinned mutable reference to `self.data`.
        let mut buffer = self.project();
        // Get the number of bytes to read from `buffer` for the current frame.
        // We read a maximum of 16KB per frame from `buffer`.
        let frame_len = buffer.len().min(MAX_FRAME_LEN);

        // Check if the buffer has any data.
        if frame_len == 0 {
            // The buffer is empty. Signal that the stream has ended.
            return Poll::Ready(None);
        }

        // Copy the bytes from the buffer into an immutable `Bytes`. This is safe
        // because `BytesMut` is only advancing an internal pointer rather than
        // moving the bytes in memory.
        let data = buffer.split_to(frame_len).freeze();
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

impl Default for Buffered {
    fn default() -> Self {
        Self::new(BytesMut::new())
    }
}

impl From<Bytes> for Buffered {
    fn from(bytes: Bytes) -> Self {
        let data = Bytes::copy_from_slice(&bytes);
        Self::new(BytesMut::from(data))
    }
}

impl From<Vec<u8>> for Buffered {
    fn from(vec: Vec<u8>) -> Self {
        let data = Bytes::from(vec);
        Self::new(BytesMut::from(data))
    }
}

impl From<&'static [u8]> for Buffered {
    fn from(slice: &'static [u8]) -> Self {
        let data = Bytes::from_static(slice);
        Self::new(BytesMut::from(data))
    }
}

impl From<String> for Buffered {
    fn from(string: String) -> Self {
        let data = Bytes::from(string.into_bytes());
        Self::new(BytesMut::from(data))
    }
}

impl From<&'static str> for Buffered {
    fn from(slice: &'static str) -> Self {
        let data = Bytes::from_static(slice.as_bytes());
        Self::new(BytesMut::from(data))
    }
}