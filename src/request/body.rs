use bytes::{Buf, Bytes, BytesMut};
use futures_core::Stream;
use hyper::body::{Body as BodyTrait, Incoming};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

/// The maximum amount of bytes that can be reserved during an allocation.
const MAX_ALLOC_SIZE: usize = isize::MAX as usize;

/// The maximum amount of bytes that can be preallocated.
const MAX_PREALLOC_SIZE: usize = 104857600; // 100 MB

#[derive(Debug)]
pub struct Body {
    inner: Box<Incoming>,
    len: Option<usize>,
}

#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: Box<Incoming>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoBytes {
    buffer: BytesMut,
    stream: BodyStream,
}

/// Preallocates a `BytesMut` buffer with the provided `capacity` if it is less
/// than or equal to `MAX_PREALLOC_SIZE`. If `capacity` is `None`, an empty
/// `BytesMut` buffer is returned.
fn bytes_mut_with_capacity(capacity: Option<usize>) -> BytesMut {
    capacity.map_or_else(BytesMut::new, |value| {
        BytesMut::with_capacity(value.min(MAX_PREALLOC_SIZE))
    })
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

impl Body {
    pub async fn into_bytes(self) -> Result<Bytes> {
        let buffer = bytes_mut_with_capacity(self.len);
        let stream = self.into_stream();

        (ReadIntoBytes { buffer, stream }).await
    }

    #[cfg(feature = "serde")]
    pub async fn into_json<T>(self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::{http::StatusCode, Error};

        let buffer = self.into_bytes().await?;

        serde_json::from_slice(&buffer).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }

    pub async fn into_string(self) -> Result<String> {
        let buffer = self.into_bytes().await?;
        Ok(String::from_utf8(Vec::from(buffer))?)
    }

    pub fn into_stream(self) -> BodyStream {
        BodyStream { body: self.inner }
    }
}

impl Body {
    pub(crate) fn new(incoming: Incoming) -> Self {
        Self {
            inner: Box::new(incoming),
            len: None,
        }
    }

    pub(crate) fn with_len(incoming: Incoming, len: usize) -> Self {
        Self {
            inner: Box::new(incoming),
            len: Some(len),
        }
    }
}

impl Stream for BodyStream {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        let mut body = Pin::new(&mut *self.body);

        loop {
            match body.as_mut().poll_frame(context) {
                // A frame was read from the body.
                Poll::Ready(Some(Ok(frame))) => {
                    if let Ok(bytes) = frame.into_data() {
                        // The frame is a data frame. Return it.
                        return Poll::Ready(Some(Ok(bytes)));
                    }
                }
                // An Error occurred when reading the next frame.
                Poll::Ready(Some(Err(error))) => {
                    let error = Error::from(error);
                    return Poll::Ready(Some(Err(error)));
                }
                // We have read all the frames from the body.
                Poll::Ready(None) => {
                    // No more frames.
                    return Poll::Ready(None);
                }
                // The body is not ready to yield a frame.
                Poll::Pending => {
                    // Wait for the next frame.
                    return Poll::Pending;
                }
            }
        }
    }
}

impl Future for ReadIntoBytes {
    type Output = Result<Bytes>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();
        let buffer = &mut this.buffer;
        let mut stream = Pin::new(&mut this.stream);

        loop {
            match stream.as_mut().poll_next(context) {
                // A frame was read from the stream.
                Poll::Ready(Some(Ok(frame))) => {
                    let frame_len = frame.len();

                    // Attempt to reserve enough capacity for the frame in the
                    // buffer if the current capacity is less than the frame
                    // length.
                    if let Err(error) = try_reserve(buffer, frame_len) {
                        // Zero out the buffer.
                        buffer.fill(0);
                        // Return the error.
                        return Poll::Ready(Err(error));
                    }

                    // Write the frame into the buffer.
                    buffer.extend_from_slice(&frame);
                }
                // An Error occurred in the underlying stream.
                Poll::Ready(Some(Err(error))) => {
                    // Zero out the buffer.
                    buffer.fill(0);
                    // Return the error and stop reading the stream.
                    return Poll::Ready(Err(error));
                }
                // We have read all the bytes from the stream.
                Poll::Ready(None) => {
                    // Copy the bytes in the buffer to a new bytes object.
                    let bytes = buffer.copy_to_bytes(buffer.len());
                    // Return the immutable, contents of buffer.
                    return Poll::Ready(Ok(bytes));
                }
                // The stream is not ready to yield a frame.
                Poll::Pending => {
                    // Wait for the next frame.
                    return Poll::Pending;
                }
            };
        }
    }
}
