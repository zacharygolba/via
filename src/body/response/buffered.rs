use bytes::{Bytes, BytesMut};
use hyper::body::{Body, Frame, SizeHint};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{Error, Result};

/// A buffered body that contains a `BytesMut` buffer. This variant is used
/// when the entire body can be buffered in memory.
pub struct Buffered {
    /// The buffer containing the body data.
    data: Box<BytesMut>,

    /// Buffered is marked as `!Unpin` because it parcitipates in an `Either`
    /// enum that contains other types that are `!Unpin`. This is a safety
    /// precaution to prevent the accidental movement of data out of the
    /// `Buffered` type when it is a variant of an `Either` enum.
    _pin: PhantomPinned,
}

impl Buffered {
    pub fn new(data: Box<BytesMut>) -> Self {
        Self {
            data,
            _pin: PhantomPinned,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Buffered {
    /// Returns a pinned mutable reference to the `data` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut BytesMut> {
        unsafe {
            //
            // Safety:
            //
            // `BytesMut` is `Unpin`. However, it is used in the context of an
            // `Either` enum that contain other types that are `!Unpin`. In order
            // to prevent the accidental movement of data out of `self` or
            // `self.data`, we mark `Buffered` as `!Unpin` and therefore need to
            // use `Pin::map_unchecked_mut` to get a pinned reference to the
            // `BytesMut` buffer.
            //
            self.map_unchecked_mut(|this| &mut *this.data)
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
        // Get a mutable reference to `self`.
        let mut buffer = self.project();
        // Get the length of the buffer. This is used to determine how
        // many bytes to copy from the buffer into the data frame.
        let len = buffer.len();

        // Check if the buffer has any data.
        if len == 0 {
            // The buffer is empty. Signal that the stream has ended.
            return Poll::Ready(None);
        }

        // Copy the bytes from the buffer into an immutable `Bytes`. This is safe
        // because `BytesMut` is only advancing an internal pointer rather than
        // moving the bytes in memory.
        let bytes = buffer.split_to(len).freeze();
        // Wrap the bytes we copied from buffer in a data frame.
        let frame = Frame::data(bytes);

        // Return the data frame to the caller.
        Poll::Ready(Some(Ok(frame)))
    }

    fn is_end_stream(&self) -> bool {
        self.is_empty()
    }

    fn size_hint(&self) -> SizeHint {
        // Get the length of the buffer and attempt to cast it to a
        // `u64`. If the cast fails, `len` will be `None`.
        let len = u64::try_from(self.len()).ok();

        // If `len` is `None`, return a size hint with no bounds. Otherwise,
        // map the remaining length to a size hint with the exact size.
        len.map_or_else(SizeHint::new, SizeHint::with_exact)
    }
}
