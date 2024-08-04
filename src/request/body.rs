use bytes::{BufMut, Bytes, BytesMut};
use futures_util::Stream;
use hyper::body::{Body as BodyTrait, Incoming};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

#[derive(Debug)]
pub struct Body {
    inner: Option<Box<Incoming>>,
    len: Option<usize>,
}

#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: Box<Incoming>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadToBytes {
    buffer: BytesMut,
    stream: BodyStream,
}

/// Conditionally reserves `additional` capacity for `bytes` if the current
/// capacity is less than additional. Returns an error if `capacity + additional`
/// would overflow `usize`.
fn try_reserve(bytes: &mut BytesMut, additional: usize) -> Result<()> {
    let capacity = bytes.capacity();

    if capacity < additional && capacity.checked_add(additional).is_some() {
        bytes.reserve(additional);
        return Ok(());
    }

    Err(Error::new(
        "failed to reserve enough capacity for the next frame.".to_owned(),
    ))
}

impl Body {
    pub async fn read_to_bytes(&mut self) -> Result<Bytes> {
        let stream = self.to_stream()?;

        if let Some(capacity) = self.len {
            ReadToBytes::with_capacity(stream, capacity).await
        } else {
            ReadToBytes::new(stream).await
        }
    }

    #[cfg(feature = "serde")]
    pub async fn read_to_json<T>(&mut self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::{http::StatusCode, Error};

        let buffer = self.read_to_bytes().await?;

        serde_json::from_slice(&buffer).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }

    pub async fn read_to_string(&mut self) -> Result<String> {
        let buffer = self.read_to_bytes().await?;
        Ok(String::from_utf8(Vec::from(buffer))?)
    }

    pub fn to_stream(&mut self) -> Result<BodyStream> {
        match self.inner.take() {
            Some(incoming) => Ok(BodyStream::new(incoming)),
            None => Err(Error::new("body has already been read".to_owned())),
        }
    }
}

impl Body {
    pub(crate) fn new(incoming: Incoming) -> Self {
        Self {
            inner: Some(Box::new(incoming)),
            len: None,
        }
    }

    pub(crate) fn with_len(incoming: Incoming, len: usize) -> Self {
        Self {
            inner: Some(Box::new(incoming)),
            len: Some(len),
        }
    }
}

impl BodyStream {
    fn new(body: Box<Incoming>) -> Self {
        Self { body }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut Incoming> {
        // Safety:
        // The `body` field is never moved out of `BodyStream`.
        unsafe {
            let this = self.get_unchecked_mut();
            Pin::new_unchecked(&mut this.body)
        }
    }
}

impl Stream for BodyStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            return match self.as_mut().project().poll_frame(context) {
                // A frame was read from the body.
                Poll::Ready(Some(Ok(frame))) => {
                    if let Ok(bytes) = frame.into_data() {
                        // The frame is a data frame.
                        Poll::Ready(Some(Ok(bytes)))
                    } else {
                        // The frame is not a data frame.
                        continue;
                    }
                }
                // An Error occurred when reading the next frame.
                Poll::Ready(Some(Err(error))) => {
                    let error = Error::from(error);
                    Poll::Ready(Some(Err(error)))
                }
                // We have read all the frames from the body.
                Poll::Ready(None) => {
                    // No more frames.
                    Poll::Ready(None)
                }
                // The body is not ready to yield a frame.
                Poll::Pending => {
                    // Wait for the next frame.
                    Poll::Pending
                }
            };
        }
    }
}

impl ReadToBytes {
    fn new(stream: BodyStream) -> Self {
        Self {
            buffer: BytesMut::new(),
            stream,
        }
    }

    fn with_capacity(stream: BodyStream, capacity: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(capacity),
            stream,
        }
    }

    fn project(self: Pin<&mut Self>) -> (Pin<&mut BodyStream>, Pin<&mut BytesMut>) {
        // Safety:
        // The `stream` and `buffer` fields are never moved out of `ReadToBytes`.
        unsafe {
            let this = self.get_unchecked_mut();
            let stream = Pin::new_unchecked(&mut this.stream);
            let buffer = Pin::new_unchecked(&mut this.buffer);

            (stream, buffer)
        }
    }
}

impl Future for ReadToBytes {
    type Output = Result<Bytes>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        loop {
            let (stream, mut buffer) = self.as_mut().project();

            return match stream.poll_next(context) {
                // A frame was read from the stream.
                Poll::Ready(Some(Ok(frame))) => {
                    let frame_len = frame.len();

                    // Attempt to reserve enough capacity for the frame in the
                    // buffer if the current capacity is less than the frame
                    // length.
                    if let Err(error) = try_reserve(&mut buffer, frame_len) {
                        // Clear the buffer.
                        buffer.clear();
                        // Return the error.
                        return Poll::Ready(Err(error));
                    }

                    // Write the frame into the buffer.
                    buffer.put(frame);

                    // Continue reading the stream.
                    continue;
                }
                // An Error occurred in the underlying stream.
                Poll::Ready(Some(Err(error))) => {
                    // Clear the buffer.
                    buffer.clear();
                    // Return the error and stop reading the stream.
                    Poll::Ready(Err(error))
                }
                // We have read all the bytes from the stream.
                Poll::Ready(None) => {
                    // Copy the bytes into a new, immutable buffer.
                    let bytes = buffer.split().freeze();
                    // Return the immutable copy of the bytes in buffer.
                    Poll::Ready(Ok(bytes))
                }
                // The stream is not ready to yield a frame.
                Poll::Pending => {
                    // Wait for the next frame.
                    Poll::Pending
                }
            };
        }
    }
}
