use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BodyStream;
use crate::Error;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoBytes {
    buffer: Vec<u8>,
    stream: BodyStream,
}

impl ReadIntoBytes {
    pub(crate) fn new(buffer: Vec<u8>, stream: BodyStream) -> Self {
        Self { buffer, stream }
    }
}

impl ReadIntoBytes {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut BodyStream>, &mut Vec<u8>) {
        let this = self.get_mut();
        let ptr = &mut this.stream;

        (Pin::new(ptr), &mut this.buffer)
    }
}

impl Future for ReadIntoBytes {
    type Output = Result<Vec<u8>, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let (mut stream, buffer) = self.project();

        loop {
            return match stream.as_mut().poll_next(context) {
                Poll::Ready(Some(Ok(frame))) => {
                    // Get a `Bytes` from the frame if it is a data frame.
                    let data = match frame.into_data() {
                        // Unwrap the bytes from the data frame.
                        Ok(bytes) => bytes,
                        // Continue to the next data frame.
                        Err(_) => continue,
                    };

                    // Get the length of the frame. If necessary, we'll use this
                    // to reserve capacity in the buffer.
                    let len = data.len();

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
                        buffer.extend_from_slice(&data);

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
