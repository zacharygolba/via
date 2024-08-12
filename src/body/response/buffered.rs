use bytes::{Buf, Bytes, BytesMut};
use hyper::body::{Body, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

/// A buffered body that contains a `BytesMut` buffer. This variant is used
/// when the entire body can be buffered in memory.
#[derive(Default)]
pub struct Buffered {
    data: Box<BytesMut>,
}

impl Buffered {
    pub fn new(data: BytesMut) -> Self {
        Self {
            data: Box::new(data),
        }
    }

    pub fn empty() -> Self {
        Default::default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl Buffered {
    /// Returns a pinned reference to the inner kind of the body.
    fn project(self: Pin<&mut Self>) -> Pin<&mut BytesMut> {
        let this = self.get_mut();
        Pin::new(&mut *this.data)
    }
}

impl Body for Buffered {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let mut buffer = self.project();
        // Get the length of the buffer. This is used to determine how
        // many bytes to copy from the buffer into the data frame.
        let len = buffer.len();

        // Check if the buffer has any data.
        if len == 0 {
            // The buffer is empty. Signal that the stream has ended.
            return Poll::Ready(None);
        }

        // Copy the bytes from the buffer into an immutable `Bytes`.
        let bytes = buffer.copy_to_bytes(len);
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
