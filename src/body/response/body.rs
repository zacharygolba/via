use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::{Buffered, Either, Mapped, Streaming};
use crate::{Error, Result};

pub struct ResponseBody {
    body: Either<Either<Buffered, Streaming>, Box<Mapped>>,
}

/// A projection type for the possible variants of the nested `Either` enums
/// that compose the `body` field of the `ResponseBody` struct.
//
// This `enum` primarily exists to get a `Pin<&mut Mapped>` reference to the
// `Box<Mapped>` contained in the `Either::Right` variant of the `body` field
// of the `ResponseBody` struct so we can delegate the call to `poll_frame` to
// `Mapped` struct when necessary since it is `!Unpin`.
enum ResponseBodyProjection<'a> {
    /// The projection type for a `Buffered` response body.
    Buffered(Pin<&'a mut Buffered>),
    /// The projection type for a `Mapped` response body.
    Mapped(Pin<&'a mut Mapped>),
    /// The projection type for a `Streaming` response body.
    Streaming(Pin<&'a mut Streaming>),
}

impl ResponseBody {
    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }

    pub fn len(&self) -> Option<usize> {
        match &self.body {
            Either::Left(Either::Left(buffered)) => Some(buffered.len()),
            Either::Right(mapped) => mapped.len(),
            _ => None,
        }
    }
}

impl ResponseBody {
    pub(crate) fn new() -> Self {
        Self::buffer(Bytes::new())
    }

    pub(crate) fn buffer(data: Bytes) -> Self {
        let buffer = Box::new(BytesMut::from(data));
        let buffered = Buffered::new(buffer);

        Self {
            body: Either::Left(Either::Left(buffered)),
        }
    }

    pub(crate) fn stream<T>(stream: T) -> Self
    where
        T: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        let stream = Streaming::new(Box::new(stream));

        Self {
            body: Either::Left(Either::Right(stream)),
        }
    }

    pub(crate) fn map<F>(self, map: Box<F>) -> Self
    where
        F: Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static,
    {
        match self.body {
            Either::Left(source) => {
                let mut mapped = Mapped::new(source);

                mapped.push(map);

                Self {
                    body: Either::Right(Box::new(mapped)),
                }
            }
            Either::Right(mut mapped) => {
                mapped.push(map);

                Self {
                    body: Either::Right(mapped),
                }
            }
        }
    }

    /// Returns a projection type of the possible variants of the nested `Either`
    /// enums that compose the `body` field.
    fn project(self: Pin<&mut Self>) -> ResponseBodyProjection {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // The only type of response body that is `Unpin` is the `Buffered`
            // struct so we have to use `get_unchecked_mut` to get a mutable
            // reference to `self`. This is safe since we're not moving any data
            // out of the pinned reference to `Self`.
            self.get_unchecked_mut()
        };

        match &mut this.body {
            Either::Left(Either::Left(buffered)) => {
                // Get a pinned reference to the `Buffered` variant.
                let pin = Pin::new(buffered);

                // Return the projection type for a `Buffered` response body.
                ResponseBodyProjection::Buffered(pin)
            }
            Either::Left(Either::Right(streaming)) => {
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The `Streaming` struct is `!Unpin` since it contains a
                    // `Box<dyn Stream>`. Since we're delegating the call to
                    // `poll_frame` directly to the `Streaming` struct, we
                    // know that creating a pinned reference to `streaming` is
                    // safe as long as the `Streaming` struct's internal API
                    // upholds the invariants of the `Pin` API.
                    Pin::new_unchecked(streaming)
                };

                // Return the projection type for a `Streaming` response body.
                ResponseBodyProjection::Streaming(pin)
            }
            Either::Right(mapped) => {
                // Get a mutable reference to the `Mapped` variant by
                // dereferencing the `&mut Box<Mapped>` to `&mut Mapped`.
                let ptr = &mut **mapped;
                // Create a `Pin` around the mutable reference at `ptr`.
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The `Mapped` struct is `!Unpin` since it may contains a
                    // `Streaming` struct which is also `!Unpin`. Since we're
                    // delegating the call to `poll_frame` directly to the
                    // `Mapped` struct, we know that creating a pinned reference
                    // to `mapped` is safe as long as the `Mapped` and `Streaming`
                    // structs' internal API upholds the invariants of the `Pin`
                    // API.
                    Pin::new_unchecked(ptr)
                };

                // Return the projection type for a `Mapped` response body.
                ResponseBodyProjection::Mapped(pin)
            }
        }
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            ResponseBodyProjection::Buffered(buffered) => buffered.poll_frame(context),
            ResponseBodyProjection::Mapped(mapped) => mapped.poll_frame(context),
            ResponseBodyProjection::Streaming(streaming) => streaming.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.body {
            Either::Left(original) => original.is_end_stream(),
            Either::Right(mapped) => mapped.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.body {
            Either::Left(original) => original.size_hint(),
            Either::Right(mapped) => mapped.size_hint(),
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
