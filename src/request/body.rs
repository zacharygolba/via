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
    pub fn into_stream(self) -> BodyStream {
        BodyStream { body: self.inner }
    }

    pub async fn read_into_bytes(self) -> Result<Bytes> {
        let buffer = bytes_mut_with_capacity(self.len);
        let stream = self.into_stream();

        (ReadIntoBytes { buffer, stream }).await
    }

    pub async fn read_into_string(self) -> Result<String> {
        let utf8 = self.read_into_vec().await?;
        Ok(String::from_utf8(utf8)?)
    }

    pub async fn read_into_vec(self) -> Result<Vec<u8>> {
        let bytes = self.read_into_bytes().await?;
        Ok(Vec::from(bytes))
    }

    #[cfg(feature = "serde")]
    pub async fn read_json<T>(self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::{http::StatusCode, Error};

        let buffer = self.read_into_bytes().await?;

        serde_json::from_slice(&buffer).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
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

impl BodyStream {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Incoming> {
        // Get a mutable reference to self.
        let this = self.get_mut();
        // Get a mutable reference to the `body` field.
        let body = &mut *this.body;

        // Return the pinned reference to the `body` field.
        Pin::new(body)
    }
}

impl Stream for BodyStream {
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context) -> Poll<Option<Self::Item>> {
        match self.project().poll_frame(context) {
            Poll::Ready(Some(Ok(frame))) => {
                if let Ok(bytes) = frame.into_data() {
                    // The frame is a data frame. Return it.
                    Poll::Ready(Some(Ok(bytes)))
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Some(Err(error))) => {
                let error = Error::from(error);
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                // No more frames.
                Poll::Ready(None)
            }
            Poll::Pending => {
                // Wait for the next frame.
                Poll::Pending
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Get the size hint from the body stream. We'll use this convert the
        // `SizeHint` type to a tuple containing the upper and lower bound of
        // the stream.
        let hint = self.body.size_hint();
        // Attempt to convert the lower bound to a usize. If the conversion
        // fails, consider the lower bound to be zero.
        let lower = usize::try_from(hint.lower()).unwrap_or(0);
        // Attempt to convert the upper bound to a usize. If the conversion
        // fails, return `None`.
        let upper = hint.upper().and_then(|value| usize::try_from(value).ok());

        (lower, upper)
    }
}

impl ReadIntoBytes {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut BytesMut>, Pin<&mut BodyStream>) {
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
