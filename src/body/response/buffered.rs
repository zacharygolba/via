use bytes::{Bytes, BytesMut};
use hyper::body::{Body, Frame, SizeHint};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// The maximum amount of data that can be read from a buffered body per frame.
const MAX_FRAME_LEN: usize = 16384; // 16KB

/// A buffered body that contains a `BytesMut` buffer. This variant is used
/// when the entire body can be buffered in memory.
pub struct Buffered {
    /// The buffer containing the body data.
    buffer: BytesMut,

    /// Buffered is marked as `!Unpin` because it parcitipates in an `Either`
    /// enum that contains other types that are `!Unpin`. This is a safety
    /// precaution to prevent the accidental movement of data out of the
    /// `Buffered` type when it is a variant of an `Either` enum.
    _pin: PhantomPinned,
}

impl Buffered {
    pub fn new(buffer: BytesMut) -> Self {
        Self {
            buffer,
            _pin: PhantomPinned,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
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
            self.map_unchecked_mut(|this| &mut this.buffer)
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
