use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::stream::BodyDataStream;
use crate::{Error, Result};

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoBytes {
    buffer: Vec<u8>,
    stream: BodyDataStream,
}

impl ReadIntoBytes {
    pub(crate) fn new(buffer: Vec<u8>, stream: BodyDataStream) -> Self {
        Self { buffer, stream }
    }
}

impl Future for ReadIntoBytes {
    type Output = Result<Vec<u8>, Error>;

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
                    if let Err(error) = buffer.try_reserve(len) {
                        // Set the buffer's length to zero.
                        buffer.clear();

                        // Return ready with the error.
                        Poll::Ready(Err(error.into()))
                    } else {
                        // Write the frame into the buffer.
                        buffer.extend_from_slice(&frame);

                        // Continue polling the stream.
                        continue;
                    }
                }
                Poll::Ready(Some(Err(error))) => {
                    // Set the buffer's length to zero.
                    buffer.clear();

                    // Return the error and stop reading the stream.
                    Poll::Ready(Err(error))
                }
                Poll::Ready(None) => {
                    let bytes = buffer.split_off(0);

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
