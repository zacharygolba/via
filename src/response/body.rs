use bytes::{Buf, Bytes};
use futures_core::Stream;
use hyper::body::{Body as BodyTrait, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

type DynStream = dyn Stream<Item = Result<Frame<Bytes>>> + Send;

pub struct Body {
    kind: BodyKind,
}

enum BodyKind {
    Buffer(Option<Bytes>),
    Stream(Box<DynStream>),
}

enum BodyKindProjection<'a> {
    Buffer(Pin<&'a mut Option<Bytes>>),
    Stream(Pin<&'a mut DynStream>),
}

struct BodyStreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    stream: T,
}

impl Body {
    pub fn len(&self) -> Option<usize> {
        self.as_buffer().map(Bytes::len)
    }

    pub fn is_empty(&self) -> bool {
        self.len().map_or(true, |len| len == 0)
    }
}

impl Body {
    pub(super) fn new() -> Self {
        let buffer = Some(Bytes::new());

        Self {
            kind: BodyKind::Buffer(buffer),
        }
    }

    pub(super) fn buffer(bytes: Bytes) -> Self {
        let buffer = Some(bytes);

        Self {
            kind: BodyKind::Buffer(buffer),
        }
    }

    pub(super) fn stream<T, D: 'static, E: 'static>(stream: T) -> Self
    where
        T: Stream<Item = Result<D, E>> + Send + 'static,
        Bytes: From<D>,
        Error: From<E>,
    {
        let stream = Box::new(BodyStreamAdapter { stream });

        Self {
            kind: BodyKind::Stream(stream),
        }
    }

    fn as_buffer(&self) -> Option<&Bytes> {
        if let BodyKind::Buffer(buffer) = &self.kind {
            buffer.as_ref()
        } else {
            None
        }
    }

    /// Returns a pinned reference to the inner kind of the body.
    fn project(self: Pin<&mut Self>) -> BodyKindProjection {
        // Safety:
        // This block is necessary because we need to project the inner kind
        // through the outer pinned reference. While the data of `Self::Buffer`
        // is `Unpin`, we don't know if the data of `Self::Stream` is `Unpin`.
        // Therefore, we need to use `unsafe` to project the inner kind.
        unsafe {
            // Get a mutable reference to `self`.
            let this = self.get_unchecked_mut();

            match &mut this.kind {
                BodyKind::Buffer(buffer) => {
                    // Create a new pinned reference to the buffer.
                    let pinned = Pin::new_unchecked(buffer);

                    // Return the projection type for `BodyKind::Buffer`.
                    BodyKindProjection::Buffer(pinned)
                }
                BodyKind::Stream(stream) => {
                    // Get a mutable reference to the stream.
                    let stream = &mut **stream;
                    // Create a new pinned reference to the stream.
                    let pinned = Pin::new_unchecked(stream);

                    // Return the projection type for `BodyKind::Stream`.
                    BodyKindProjection::Stream(pinned)
                }
            }
        }
    }
}

impl From<()> for Body {
    fn from(_: ()) -> Self {
        Self::new()
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Self::buffer(bytes)
    }
}

impl From<Vec<u8>> for Body {
    fn from(vec: Vec<u8>) -> Self {
        Self::buffer(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for Body {
    fn from(slice: &'static [u8]) -> Self {
        Self::buffer(Bytes::from_static(slice))
    }
}

impl From<String> for Body {
    fn from(string: String) -> Self {
        Self::buffer(Bytes::from(string))
    }
}

impl From<&'static str> for Body {
    fn from(slice: &'static str) -> Self {
        Self::buffer(Bytes::from_static(slice.as_bytes()))
    }
}

impl BodyTrait for Body {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            BodyKindProjection::Buffer(mut buffer) => {
                // Take the buffer and map it to a data frame and return it.
                Poll::Ready(buffer.take().map(|bytes| Ok(Frame::data(bytes))))
            }
            BodyKindProjection::Stream(mut stream) => {
                // Poll the stream for the next frame and return the result.
                stream.as_mut().poll_next(context)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        matches!(self.kind, BodyKind::Buffer(None))
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            BodyKind::Buffer(Some(bytes)) => SizeHint::with_exact(bytes.remaining() as u64),
            BodyKind::Buffer(None) => SizeHint::new(),
            BodyKind::Stream(stream) => {
                let (lower, upper) = stream.size_hint();
                let mut size_hint = SizeHint::new();

                size_hint.set_lower(lower as u64);

                if let Some(value) = upper {
                    size_hint.set_upper(value as u64);
                }

                size_hint
            }
        }
    }
}

impl<T, D, E> BodyStreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // Safety:
        // This block is necessary because we need to project the inner stream
        // through the outer pinned reference. We don't know if `T` is `Unpin`
        // so we need to use `unsafe` to create the pinned reference with
        // `Pin::new_unchecked`.
        unsafe {
            // Get a mutable reference to `self`.
            let this = self.get_unchecked_mut();
            // Get a mutable reference to the `stream` field.
            let stream = &mut this.stream;

            // Return the pinned reference to the `stream` field.
            Pin::new_unchecked(stream)
        }
    }
}

impl<T, D, E> Stream for BodyStreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().poll_next(context) {
            Poll::Ready(Some(Ok(data))) => {
                // Convert the data to bytes.
                let bytes = Bytes::from(data);
                // Wrap the bytes in a data frame.
                let frame = Frame::data(bytes);
                // Yield the data frame.
                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(error))) => {
                // An error occurred while reading the stream. Wrap the
                // error with `via::Error`.
                let error = Error::from(error);
                // Yield the wrapped error.
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                // The stream has ended.
                Poll::Ready(None)
            }
            Poll::Pending => {
                // The stream is not ready to yield a frame.
                Poll::Pending
            }
        }
    }
}
