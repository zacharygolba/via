use bytes::{Buf, Bytes};
use futures_util::Stream;
use hyper::body::{Body as BodyTrait, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

pub type Frame = hyper::body::Frame<Bytes>;
type DynStream = dyn Stream<Item = Result<Frame>> + Send + 'static;

pub struct Body {
    kind: BodyKind,
}

enum BodyKind {
    Buffer(Box<Bytes>),
    Stream(Box<DynStream>),
}

enum BodyKindProject<'a> {
    Buffer(Pin<&'a mut Bytes>),
    Stream(Pin<&'a mut DynStream>),
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
        let bytes = Box::new(Bytes::new());

        Self {
            kind: BodyKind::Buffer(bytes),
        }
    }

    pub(super) fn buffer(bytes: Bytes) -> Self {
        let bytes = Box::new(bytes);

        Self {
            kind: BodyKind::Buffer(bytes),
        }
    }

    pub(super) fn stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame>> + Send + 'static,
    {
        let stream = Box::new(stream);

        Self {
            kind: BodyKind::Stream(stream),
        }
    }

    fn as_buffer(&self) -> Option<&Bytes> {
        if let BodyKind::Buffer(bytes) = &self.kind {
            Some(bytes)
        } else {
            None
        }
    }

    fn project(self: Pin<&mut Self>) -> BodyKindProject {
        // Safety:
        // This block is necessary because we need to project the pin through
        // the different variants of the enum. The `unsafe` block ensures that
        // we can safely create a new `Pin` to the inner data without violating
        // the guarantees of the `Pin` API.
        unsafe {
            let this = self.get_unchecked_mut();

            match &mut this.kind {
                BodyKind::Buffer(bytes) => {
                    // Get a mutable reference to the inner `Bytes` from the box.
                    // This is safe because `self` is pinned, and `kind` is never
                    // moved out of `Body`.
                    let ptr = &mut **bytes;

                    // Return the pinned reference to the inner `Bytes`.
                    BodyKindProject::Buffer(Pin::new_unchecked(ptr))
                }
                BodyKind::Stream(stream) => {
                    // Get a mutable reference to the inner `DynStream` from the
                    // box. This is safe because `self` is pinned, and `kind` is
                    // never moved out of `Body`.
                    let ptr = &mut **stream;

                    // Return the pinned reference to the inner `DynStream`.
                    BodyKindProject::Stream(Pin::new_unchecked(ptr))
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
    ) -> Poll<Option<Result<Frame, Self::Error>>> {
        match self.project() {
            // The body is a buffer.
            BodyKindProject::Buffer(mut bytes) => {
                // Get remaining length of the buffer.
                let remaining = bytes.remaining();
                // Create a new data frame from the remaining bytes.
                let frame = Frame::data(bytes.split_to(remaining));
                // Return the remaining bytes of the buffer as a data frame.
                Poll::Ready(Some(Ok(frame)))
            }
            // The body is a stream.
            BodyKindProject::Stream(stream) => {
                // Delegate to the stream to poll the next frame.
                stream.poll_next(context)
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            BodyKind::Buffer(bytes) => bytes.is_empty(),
            BodyKind::Stream(_) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            BodyKind::Buffer(bytes) => SizeHint::with_exact(bytes.remaining() as u64),
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
