use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::stream::BodyDataStream;
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
}

impl Future for ReadIntoBytes {
    type Output = Result<Bytes>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        let buffer = &mut this.buffer;
        let mut stream = Pin::new(&mut this.stream);

        loop {
            return match stream.as_mut().poll_next(context) {
                Poll::Ready(Some(Ok(frame))) => {
                    // Get the length of the frame. If necessary, we'll use this
                    // to reserve capacity in the buffer.
                    let len = frame.len();

                    // Attempt to reserve enough capacity for the frame in the
                    // buffer if the current capacity is less than the frame
                    // length.
                    if let Err(error) = try_reserve(buffer, len) {
                        // Zero out the buffer.
                        buffer.fill(0);

                        // Set the buffer's length to zero.
                        buffer.clear();

                        // Return ready with the error.
                        Poll::Ready(Err(error))
                    } else {
                        // Write the frame into the buffer.
                        buffer.extend_from_slice(&frame);

                        // Continue polling the stream.
                        continue;
                    }
                }
                Poll::Ready(Some(Err(error))) => {
                    // Zero out the buffer.
                    buffer.fill(0);

                    // Set the buffer's length to zero.
                    buffer.clear();

                    // Return the error and stop reading the stream.
                    Poll::Ready(Err(error))
                }
                Poll::Ready(None) => {
                    // Get the number of bytes to read from the buffer.
                    let len = buffer.len();
                    // Read `len` bytes from `buffer` into an immutable `Bytes`.
                    let bytes = buffer.split_to(len).freeze();

                    // Return the immutable, contents of buffer.
                    Poll::Ready(Ok(bytes))
                }
                Poll::Pending => {
                    // Wait for the next frame.
                    Poll::Pending
                }
            };
        }
    }
}
