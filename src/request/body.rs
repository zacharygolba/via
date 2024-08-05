use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body as BodyTrait, Incoming};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

const MAX_BUFFER_SIZE: usize = isize::MAX as usize;

#[derive(Debug)]
pub struct Body {
    inner: Box<Incoming>,
    len: Option<usize>,
}

#[must_use = "streams do nothing unless polled"]
pub struct BodyStream {
    body: Pin<Box<Incoming>>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct ReadIntoBytes {
    buffer: Vec<u8>,
    stream: BodyStream,
}

/// Conditionally reserves `additional` capacity for `bytes` if the current
/// capacity is less than additional. Returns an error if `capacity + additional`
/// would overflow `isize`.
fn try_reserve(bytes: &mut Vec<u8>, additional: usize) -> Result<()> {
    let capacity = bytes.capacity();

    if capacity >= additional {
        // The buffer has enough capacity. Return without reallocating.
        return Ok(());
    }

    match capacity.checked_add(additional) {
        Some(total) if total <= MAX_BUFFER_SIZE => {
            bytes.reserve(additional);
            Ok(())
        }
        _ => Err(Error::new(
            "failed to reserve enough capacity for the next frame.".to_owned(),
        )),
    }
}

impl Body {
    pub async fn into_bytes(self) -> Result<Vec<u8>> {
        let buffer = self.len.map(Vec::with_capacity).unwrap_or_default();
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
        let string = String::from_utf8(buffer)?;

        Ok(string)
    }

    pub fn into_stream(self) -> BodyStream {
        BodyStream {
            body: Box::into_pin(self.inner),
        }
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
        loop {
            match self.body.as_mut().poll_frame(context) {
                // A frame was read from the body.
                Poll::Ready(Some(Ok(frame))) => {
                    if let Ok(bytes) = frame.into_data() {
                        // The frame is a data frame.
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
            };
        }
    }
}

impl Future for ReadIntoBytes {
    type Output = Result<Vec<u8>>;

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
                    // Take ownership of the buffer.
                    let buffer = std::mem::take(buffer);
                    // Return the owned buffer.
                    return Poll::Ready(Ok(buffer));
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
