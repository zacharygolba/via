use bytes::{Buf, Bytes, BytesMut};
use futures_core::Stream;
use hyper::body::{Frame, SizeHint};
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use super::StreamAdapter;
use crate::{body::size_hint, Error, Result};

type DynStream = dyn Stream<Item = Result<Frame<Bytes>>> + Send;

pub struct ResponseBody {
    /// The kind of body that is being used. This allows us to support both
    /// buffered and streaming response bodies.
    kind: ResponseBodyKind,

    /// A marker field that is used to indicate that `Body` is `!Unpin`. This
    /// is necessary because `Body` may contain a stream that is not `Unpin`.
    _pin: PhantomPinned,
}

/// An enum that represents the different kinds of bodies that can be used in
/// a response. This allows us to support both buffered and streaming response
/// bodies.
enum ResponseBodyKind {
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
enum ResponseBodyKindProjection<'a> {
    Buffer(Pin<&'a mut BytesMut>),
    Stream(Pin<&'a mut DynStream>),
}

impl ResponseBody {
    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }

    pub fn len(&self) -> Option<usize> {
        self.as_buffer().map(BytesMut::len)
    }
}

impl ResponseBody {
    pub(crate) fn new() -> Self {
        let buffer = Box::new(BytesMut::new());

        Self {
            kind: ResponseBodyKind::Buffer(buffer),
            _pin: PhantomPinned,
        }
    }

    pub(crate) fn buffer(bytes: Bytes) -> Self {
        let buffer = Box::new(BytesMut::from(bytes));

        Self {
            kind: ResponseBodyKind::Buffer(buffer),
            _pin: PhantomPinned,
        }
    }

    pub(crate) fn stream<T, D: 'static, E: 'static>(stream: T) -> Self
    where
        T: Stream<Item = Result<D, E>> + Send + 'static,
        Bytes: From<D>,
        Error: From<E>,
    {
        let stream = Box::new(StreamAdapter::new(stream));

        Self {
            kind: ResponseBodyKind::Stream(stream),
            _pin: PhantomPinned,
        }
    }

    // TODO:
    // Determine if there is a way we can compose closures of nested map
    // functions to avoid recursive pointers as the value of `BodyKind::Stream`.
    //
    // Perhaps we can use a `Box<dyn Fn(Bytes) -> Result<Bytes, E> + Send>` to
    // store the map functions and unwind them in the `poll_frame` method.
    pub(crate) fn map<F, E: 'static>(self, _: F) -> Self
    where
        F: Fn(Bytes) -> Result<Bytes, E> + Send + 'static,
        Error: From<E>,
    {
        todo!()
        // let stream = Box::new(MapBody::new(self, map));

        // Body {
        //     kind: BodyKind::Stream(stream),
        //     _pin: PhantomPinned,
        // }
    }

    fn as_buffer(&self) -> Option<&BytesMut> {
        if let ResponseBodyKind::Buffer(buffer) = &self.kind {
            Some(buffer)
        } else {
            None
        }
    }

    /// Returns a pinned reference to the inner kind of the body.
    fn project(self: Pin<&mut Self>) -> ResponseBodyKindProjection {
        let this = unsafe {
            // Safety:
            // This block is necessary because we need to get a mutable reference
            // to `self` through the pinned reference. Since `self.kind` may be
            // `BodyKind::Stream` and wrap a type that is not `Unpin`, we need to
            // use `unsafe` to get a mutable reference to `self`.
            self.get_unchecked_mut()
        };

        match &mut this.kind {
            ResponseBodyKind::Buffer(buffer) => {
                // Deref the boxed bytes to get a mutable reference to the
                // contained buffer.
                let ptr = &mut **buffer;
                // The `BodyKind::Buffer` variant wraps a `Box<BytesMut>` which
                // is `Unpin`. We can safely create a pinned reference to the
                // buffer without using `Pin::new_unchecked`.
                let pin = Pin::new(ptr);

                // Return the projection type for `BodyKind::Buffer`.
                ResponseBodyKindProjection::Buffer(pin)
            }
            ResponseBodyKind::Stream(stream) => {
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
                ResponseBodyKindProjection::Stream(pin)
            }
        }
    }
}

impl From<()> for ResponseBody {
    fn from(_: ()) -> Self {
        Self::new()
    }
}

impl From<Bytes> for ResponseBody {
    fn from(bytes: Bytes) -> Self {
        Self::buffer(bytes)
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(vec: Vec<u8>) -> Self {
        Self::buffer(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for ResponseBody {
    fn from(slice: &'static [u8]) -> Self {
        Self::buffer(Bytes::from_static(slice))
    }
}

impl From<String> for ResponseBody {
    fn from(string: String) -> Self {
        Self::buffer(Bytes::from(string))
    }
}

impl From<&'static str> for ResponseBody {
    fn from(slice: &'static str) -> Self {
        Self::buffer(Bytes::from_static(slice.as_bytes()))
    }
}

impl hyper::body::Body for ResponseBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            ResponseBodyKindProjection::Buffer(mut buffer) => {
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
            ResponseBodyKindProjection::Stream(stream) => {
                // Poll the stream for the next frame.
                stream.poll_next(context)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            ResponseBodyKind::Buffer(buffer) => buffer.is_empty(),
            ResponseBodyKind::Stream(_) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            ResponseBodyKind::Buffer(buffer) => {
                // Get the length of the buffer and attempt to cast it to a
                // `u64`. If the cast fails, `len` will be `None`.
                let len = u64::try_from(buffer.len()).ok();

                // If `len` is `None`, return a size hint with no bounds. Otherwise,
                // map the remaining length to a size hint with the exact size.
                len.map_or_else(SizeHint::new, SizeHint::with_exact)
            }
            ResponseBodyKind::Stream(stream) => {
                // Delegate the call to the stream to get the size hint and use
                // the helper function to adapt the returned tuple to a
                // `SizeHint`.
                size_hint::from_stream_for_body(&**stream)
            }
        }
    }
}
