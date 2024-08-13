use bytes::{Buf, Bytes, BytesMut};
use futures_core::Stream;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use super::BodyDataStream;
use crate::{Error, Result};

/// The maximum amount of bytes that can be reserved during an allocation.
const MAX_ALLOC_SIZE: usize = isize::MAX as usize;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoBytes {
    buffer: BytesMut,
    stream: BodyDataStream,
}

/// Conditionally reserves `additional` capacity for `bytes` if the current
/// capacity is less than additional. Returns an error if `capacity + additional`
/// would overflow `isize`.
fn try_reserve(bytes: &mut BytesMut, additional: usize) -> Result<()> {
    let capacity = bytes.capacity();

    if capacity >= additional {
        // The buffer has enough capacity. Return without reallocating.
        return Ok(());
    }

    match capacity.checked_add(additional) {
        Some(total) if total <= MAX_ALLOC_SIZE => {
            bytes.reserve(additional);
            Ok(())
        }
        _ => Err(Error::new(
            "failed to reserve enough capacity for the next frame.".to_owned(),
        )),
    }
}

impl ReadIntoBytes {
    pub(crate) fn new(buffer: BytesMut, stream: BodyDataStream) -> Self {
        Self { buffer, stream }
    }

    fn project(self: Pin<&mut Self>) -> (Pin<&mut BytesMut>, Pin<&mut BodyDataStream>) {
        // Get a mutable reference to self.
        let this = self.get_mut();
        let buffer = &mut this.buffer;
        let stream = &mut this.stream;

        // Project the buffer and stream.
        (Pin::new(buffer), Pin::new(stream))
    }
}

impl Future for ReadIntoBytes {
    type Output = Result<Bytes>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let (mut buffer, mut stream) = self.project();

        loop {
            match stream.as_mut().poll_next(context) {
                Poll::Ready(Some(Ok(frame))) => {
                    let frame_len = frame.len();

                    // Attempt to reserve enough capacity for the frame in the
                    // buffer if the current capacity is less than the frame
                    // length.
                    if let Err(error) = try_reserve(&mut buffer, frame_len) {
                        // Zero out the buffer.
                        buffer.fill(0);

                        // Set the buffer's length to zero.
                        buffer.clear();

                        // Return the error.
                        return Poll::Ready(Err(error));
                    }

                    // Write the frame into the buffer.
                    buffer.extend_from_slice(&frame);
                }
                Poll::Ready(Some(Err(error))) => {
                    // Zero out the buffer.
                    buffer.fill(0);

                    // Set the buffer's length to zero.
                    buffer.clear();

                    // Return the error and stop reading the stream.
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(None) => {
                    let buffer_len = buffer.len();

                    if buffer_len == 0 {
                        return Poll::Ready(Ok(Bytes::new()));
                    }

                    // Copy the bytes in the buffer to a new bytes object.
                    let bytes = buffer.copy_to_bytes(buffer_len);

                    // Return the immutable, contents of buffer.
                    return Poll::Ready(Ok(bytes));
                }
                Poll::Pending => {
                    // Wait for the next frame.
                    return Poll::Pending;
                }
            }
        }
    }
}
