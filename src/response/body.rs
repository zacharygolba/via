use bytes::Bytes;
use futures_util::Stream;
use hyper::body::{Body as BodyTrait, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

pub type Frame = hyper::body::Frame<Bytes>;
type DynStream = dyn Stream<Item = Result<Frame>> + Send + 'static;

pub enum Body {
    Buffer(Option<Pin<Box<Bytes>>>),
    Stream(Pin<Box<DynStream>>),
}

enum BodyProject<'a> {
    Buffer(Option<Pin<&'a mut Bytes>>),
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
        Self::Buffer(None)
    }

    pub(super) fn buffer(bytes: Bytes) -> Self {
        Self::Buffer(Some(Box::pin(bytes)))
    }

    pub(super) fn stream<S>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame>> + Send + 'static,
    {
        Self::Stream(Box::pin(stream))
    }

    fn as_buffer(&self) -> Option<&Bytes> {
        if let Self::Buffer(option) = self {
            option.as_ref().map(|bytes| bytes.as_ref().get_ref())
        } else {
            None
        }
    }

    fn project(self: Pin<&mut Self>) -> BodyProject {
        // Safety:
        // This block is necessary because we need to project the pin through
        // the different variants of the enum. The `unsafe` block ensures that
        // we can safely create a new `Pin` to the inner data without violating
        // the guarantees of the `Pin` API.
        unsafe {
            match self.get_unchecked_mut() {
                Self::Buffer(buffer) => {
                    // Map `Option<Pin<Box<Bytes>>>` to `Option<Pin<&mut Bytes>>`.
                    let projected = buffer.as_mut().map(|bytes| {
                        // Get a mutable reference to the inner `Bytes` from the
                        // pinned box. This is safe because the box is pinned,
                        // so the `Bytes` cannot move.
                        let ptr = bytes.as_mut().get_unchecked_mut();

                        // Create a new `Pin` from the mutable reference to
                        // the `Bytes`.
                        Pin::new_unchecked(ptr)
                    });

                    // Return the option containing the pin projection
                    // wrapped in `BodyProject::Buffer`.
                    BodyProject::Buffer(projected)
                }
                Self::Stream(stream) => {
                    // Get a mutable reference to the inner `DynStream` from the
                    // pinned box. This is safe because the box is pinned, so
                    // the `DynStream` cannot move.
                    let ptr = stream.as_mut().get_unchecked_mut();

                    // Create a new `Pin` from the mutable reference to the `DynStream`.
                    BodyProject::Stream(Pin::new_unchecked(ptr))
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
            BodyProject::Buffer(None) => Poll::Ready(None),
            BodyProject::Buffer(Some(mut bytes)) => {
                let bytes = bytes.split_off(0);
                let frame = Frame::data(bytes);

                Poll::Ready(Some(Ok(frame)))
            }
            BodyProject::Stream(stream) => stream.poll_next(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Buffer(buffer) => buffer.is_none(),
            Self::Stream(_) => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self {
            Self::Buffer(Some(bytes)) => SizeHint::with_exact(bytes.len() as u64),
            Self::Buffer(None) => SizeHint::new(),
            Self::Stream(stream) => {
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
