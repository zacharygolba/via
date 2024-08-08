use bytes::{Buf, Bytes, BytesMut};
use futures_core::Stream;
use hyper::body::{Body as BodyTrait, Frame, SizeHint};
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

type DynStream = dyn Stream<Item = Result<Frame<Bytes>>> + Send;

pub struct Body {
    /// The kind of body that is being used. This allows us to support both
    /// buffered and streaming response bodies.
    kind: BodyKind,

    /// A marker field that is used to indicate that `Body` is `!Unpin`. This
    /// is necessary because `Body` may contain a stream that is not `Unpin`.
    _pin: PhantomPinned,
}

/// An enum that represents the different kinds of bodies that can be used in
/// a response. This allows us to support both buffered and streaming response
/// bodies.
enum BodyKind {
    /// A buffered body that contains a `BytesMut` buffer. This variant is used
    /// when the entire body can be buffered in memory.
    Buffer(Box<BytesMut>),

    /// A stream body that contains a `DynStream` trait object. This variant is
    /// used when the body is too large to be buffered in memory or when the
    /// each frame of the body needs to be processed as it is received.
    Stream(Box<DynStream>),
}

/// A projection type of the `BodyKind` enum that allows for the inner kind to
/// participate in API calls that require a pinned reference.
enum BodyKindProjection<'a> {
    Buffer(Pin<&'a mut BytesMut>),
    Stream(Pin<&'a mut DynStream>),
}

/// A stream adapter that converts a stream of `Result<D, E>` into a stream of
/// `Result<Frame<Bytes>>`. This adapter allows for response bodies to be built
/// from virtually any stream that yields data that can be converted into bytes.
#[must_use = "streams do nothing unless polled"]
struct BodyStreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    /// The `Stream` that we are adapting to yield `Result<Frame<Bytes>>`.
    stream: T,

    /// This field is used to mark `BodyStreamAdapter` as `!Unpin`. This is
    /// necessary because `T` may not be `Unpin` and we need to project the
    /// `stream` through a pinned reference to `Self` so it can be polled.
    _pin: PhantomPinned,
}

impl Body {
    pub fn len(&self) -> Option<usize> {
        self.as_buffer().map(BytesMut::len)
    }

    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }
}

impl Body {
    pub(super) fn new() -> Self {
        let buffer = Box::new(BytesMut::new());

        Self {
            kind: BodyKind::Buffer(buffer),
            _pin: PhantomPinned,
        }
    }

    pub(super) fn buffer(bytes: Bytes) -> Self {
        let buffer = Box::new(BytesMut::from(bytes));

        Self {
            kind: BodyKind::Buffer(buffer),
            _pin: PhantomPinned,
        }
    }

    pub(super) fn stream<T, D: 'static, E: 'static>(stream: T) -> Self
    where
        T: Stream<Item = Result<D, E>> + Send + 'static,
        Bytes: From<D>,
        Error: From<E>,
    {
        let stream = Box::new(BodyStreamAdapter::new(stream));

        Self {
            kind: BodyKind::Stream(stream),
            _pin: PhantomPinned,
        }
    }

    fn as_buffer(&self) -> Option<&BytesMut> {
        if let BodyKind::Buffer(buffer) = &self.kind {
            Some(buffer)
        } else {
            None
        }
    }

    /// Returns a pinned reference to the inner kind of the body.
    fn project(self: Pin<&mut Self>) -> BodyKindProjection {
        let this = unsafe {
            // Safety:
            // This block is necessary because we need to get a mutable reference
            // to `self` through the pinned reference. Since `self.kind` may be
            // `BodyKind::Stream` and wrap a type that is not `Unpin`, we need to
            // use `unsafe` to get a mutable reference to `self`.
            self.get_unchecked_mut()
        };

        match &mut this.kind {
            BodyKind::Buffer(buffer) => {
                // Deref the boxed bytes to get a mutable reference to the
                // contained buffer.
                let ptr = &mut **buffer;
                // The `BodyKind::Buffer` variant wraps a `Box<BytesMut>` which
                // is `Unpin`. We can safely create a pinned reference to the
                // buffer without using `Pin::new_unchecked`.
                let pin = Pin::new(ptr);

                // Return the projection type for `BodyKind::Buffer`.
                BodyKindProjection::Buffer(pin)
            }
            BodyKind::Stream(stream) => {
                // Deref the boxed stream to get a mutable reference to the
                // contained stream.
                let ptr = &mut **stream;
                // Construct a pinned reference around our mutable reference to
                // `self.stream` using `Pin::new_unchecked`.
                let pin = unsafe {
                    // Safety:
                    // We know that `self.stream` is `!Unpin`. Therefore, we need
                    // to use `unsafe` to create a pinned reference to it.
                    Pin::new_unchecked(ptr)
                };

                // Return the projection type for `BodyKind::Stream`.
                BodyKindProjection::Stream(pin)
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
                // Get the length of the buffer. This is used to determine how
                // many bytes to copy from the buffer into the data frame.
                let len = buffer.len();

                // Check if the buffer has any data.
                if len == 0 {
                    // The buffer is empty. Signal that the stream has ended.
                    return Poll::Ready(None);
                }

                // Copy the bytes from the buffer into an immutable `Bytes`.
                let bytes = buffer.copy_to_bytes(len);
                // Wrap the bytes we copied from buffer in a data frame.
                let frame = Frame::data(bytes);

                // Return the data frame to the caller.
                Poll::Ready(Some(Ok(frame)))
            }
            BodyKindProjection::Stream(stream) => {
                // Poll the stream for the next frame.
                stream.poll_next(context)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            BodyKind::Buffer(buffer) => buffer.is_empty(),
            BodyKind::Stream(_) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            BodyKind::Buffer(buffer) => {
                // Get the length of the buffer and attempt to cast it to a
                // `u64`. If the cast fails, `len` will be `None`.
                let len = u64::try_from(buffer.len()).ok();

                // If `len` is `None`, return a size hint with no bounds. Otherwise,
                // map the remaining length to a size hint with the exact size.
                len.map_or_else(SizeHint::new, SizeHint::with_exact)
            }
            BodyKind::Stream(stream) => {
                // Get a tuple containing the lower and upper bounds of the
                // stream. The `BodyTrait` uses the `SizeHint` type rather than
                // a tuple so we need to convert the returned tuple instead of
                // simply delegating the call to the stream.
                let (lower, upper) = stream.size_hint();
                // Create a new `SizeHint` with the default values. We will set
                // the lower and upper bounds individually since we don't know
                // if the stream has an upper bound.
                let mut size_hint = SizeHint::new();

                // Attempt to cast the lower bound to a `u64`. If the cast fails,
                // do not set the lower bound of the size hint.
                if let Some(value) = u64::try_from(lower).ok() {
                    size_hint.set_lower(value);
                }

                // Check if the upper bound is `Some`. If it is, attempt to cast
                // it to a `u64`. If the cast fails or the upper bound is `None`,
                // do not set the upper bound of the size hint.
                if let Some(value) = upper.and_then(|value| u64::try_from(value).ok()) {
                    size_hint.set_upper(value);
                }

                // Return the size hint.
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
    fn new(stream: T) -> Self {
        Self {
            stream,
            _pin: PhantomPinned,
        }
    }

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Get the size hint from the inner stream.
        self.stream.size_hint()
    }
}
