use bytes::{Bytes, BytesMut};
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// The maximum amount of data that can be read from a buffered body per frame.
const MAX_FRAME_LEN: usize = 16384; // 16KB

/// An optimized body that is used when the entire body is already in memory.
#[derive(Debug)]
pub struct Buffered {
    /// The buffer containing the body data.
    data: Pin<Box<BytesMut>>,
}

impl Buffered {
    pub fn new(data: Bytes) -> Self {
        Self {
            data: Box::pin(BytesMut::from(data)),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Body for Buffered {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // Get a mutable reference to `self`. This is safe because `poll_frame`
        // doesn't move any data out of `self` or it's fields.
        // Get the number of bytes to read from `buffer` for the current frame.
        // We read a maximum of 16KB per frame from `buffer`.
        let len = self.data.len().min(MAX_FRAME_LEN);

        // Check if the buffer has any data.
        if len == 0 {
            // The buffer is empty. Signal that the stream has ended.
            return Poll::Ready(None);
        }

        // Copy the bytes from the buffer into an immutable `Bytes`. This is safe
        // because `BytesMut` is only advancing an internal pointer rather than
        // moving the bytes in memory.
        let data = self.data.split_to(len).freeze();
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
        Self::new(Bytes::new())
    }
}

impl From<()> for Buffered {
    fn from(_: ()) -> Self {
        Buffered::default()
    }
}

impl From<Bytes> for Buffered {
    fn from(bytes: Bytes) -> Self {
        Buffered::new(bytes)
    }
}

impl From<Vec<u8>> for Buffered {
    fn from(vec: Vec<u8>) -> Self {
        let data = Bytes::from(vec);
        Buffered::new(data)
    }
}

impl From<&'static [u8]> for Buffered {
    fn from(slice: &'static [u8]) -> Self {
        let data = Bytes::from_static(slice);
        Buffered::new(data)
    }
}

impl From<String> for Buffered {
    fn from(string: String) -> Self {
        // Delegate `From<String>` to `From<Vec<u8>>`.
        string.into_bytes().into()
    }
}

impl From<&'static str> for Buffered {
    fn from(slice: &'static str) -> Self {
        // Delegate `From<&'static str>` to `From<&'static [u8]>`.
        slice.as_bytes().into()
    }
}
