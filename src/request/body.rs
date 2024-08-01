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
    inner: Option<Pin<Box<Incoming>>>,
    len: Option<usize>,
}

#[derive(Debug)]
pub struct BodyStream {
    body: Pin<Box<Incoming>>,
}

#[derive(Debug)]
pub struct ReadToBytes {
    buffer: Option<BytesMut>,
    stream: BodyStream,
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
            Some(body) => Ok(BodyStream::new(body)),
            None => Err(Error::new("body has already been read".to_owned())),
        }
    }
}

impl Body {
    pub(crate) fn new(inner: Incoming) -> Self {
        Self {
            inner: Some(Box::pin(inner)),
            len: None,
        }
    }

    pub(crate) fn with_len(inner: Incoming, len: usize) -> Self {
        Self {
            inner: Some(Box::pin(inner)),
            len: Some(len),
        }
    }
}

impl BodyStream {
    fn new(body: Pin<Box<Incoming>>) -> Self {
        Self { body }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut Incoming> {
        // Safety: `body` is never moved after being pinned.
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
            match self.as_mut().project().poll_frame(context) {
                Poll::Ready(Some(Ok(frame))) if frame.is_data() => {
                    let bytes = frame.into_data().unwrap();
                    return Poll::Ready(Some(Ok(bytes)));
                }
                Poll::Ready(Some(Err(error))) => {
                    let error = Error::from(error);
                    return Poll::Ready(Some(Err(error)));
                }
                Poll::Ready(Some(Ok(_))) => {
                    // Skip trailers.
                    continue;
                }
                Poll::Ready(None) => {
                    // No more frames.
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    // Wait for the next frame.
                    return Poll::Pending;
                }
            }
        }
    }
}

impl ReadToBytes {
    fn new(stream: BodyStream) -> Self {
        Self {
            buffer: Some(BytesMut::new()),
            stream,
        }
    }

    fn with_capacity(stream: BodyStream, capacity: usize) -> Self {
        Self {
            buffer: Some(BytesMut::with_capacity(capacity)),
            stream,
        }
    }

    fn project(self: Pin<&mut Self>) -> (Pin<&mut BodyStream>, Pin<&mut Option<BytesMut>>) {
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
            let (stream, option) = self.as_mut().project();

            match stream.poll_next(context) {
                Poll::Ready(Some(Ok(frame))) => {
                    let frame_len = frame.len();
                    let buffer = match option.get_mut() {
                        Some(buffer) => buffer,
                        None => {
                            return Poll::Ready(Err(Error::new(
                                "buffer has already been taken".to_owned(),
                            )));
                        }
                    };

                    if buffer.capacity() < frame_len {
                        buffer.reserve(frame_len);
                    }

                    buffer.put(frame);
                }
                Poll::Ready(Some(Err(error))) => {
                    let error = Error::from(error);
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(None) => {
                    let bytes = match option.get_mut().take() {
                        Some(buffer) => buffer.freeze(),
                        None => {
                            return Poll::Ready(Err(Error::new(
                                "buffer has already been taken".to_owned(),
                            )));
                        }
                    };

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
