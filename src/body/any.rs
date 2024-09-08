use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::Poll;

use super::{Boxed, Buffered};
use crate::Error;

/// A sum type that representing any type of body.
#[non_exhaustive]
#[must_use = "streams do nothing unless polled"]
pub enum AnyBody {
    Dyn(Boxed),
    Buf(Buffered),
}

enum AnyBodyProjection<'a> {
    Dyn(Pin<&'a mut Boxed>),
    Buf(Pin<&'a mut Buffered>),
}

impl AnyBody {
    pub fn new() -> Self {
        Self::Buf(Default::default())
    }
}

impl AnyBody {
    fn project(self: Pin<&mut Self>) -> AnyBodyProjection {
        let this = unsafe {
            //
            // Safety:
            //
            // We need a mutable reference to self to project the inner body type
            // for the variant that is currently stored in the enum. The pointer
            // is never moved or deallocated for any of the variants.
            //
            self.get_unchecked_mut()
        };

        match this {
            Self::Dyn(ptr) => {
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The implementation of `Body::poll_frame` for `Boxed`
                    // operates directly on a `Pin<Box<dyn Body>>` and does not
                    // move the pointer or the boxed value out of the pinned ref.
                    //
                    Pin::new_unchecked(ptr)
                };

                AnyBodyProjection::Dyn(pin)
            }
            Self::Buf(ptr) => {
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The implementation of `Body::poll_frame` for `Buffered`
                    // treats the buffer as pinned and does not move data out of
                    // the buffer or the pinned mutable reference. Instead, data
                    // is copied out of the buffer and the cursor is advanced to
                    // the next frame.
                    //
                    Pin::new_unchecked(ptr)
                };

                AnyBodyProjection::Buf(pin)
            }
        }
    }
}

impl Body for AnyBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            AnyBodyProjection::Dyn(boxed) => boxed.poll_frame(context),
            AnyBodyProjection::Buf(buffered) => buffered.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Dyn(boxed) => boxed.is_end_stream(),
            Self::Buf(buffered) => buffered.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Dyn(boxed) => boxed.size_hint(),
            Self::Buf(buffered) => buffered.size_hint(),
        }
    }
}

impl Default for AnyBody {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Boxed> for AnyBody {
    fn from(boxed: Boxed) -> Self {
        Self::Dyn(boxed)
    }
}

impl<T> From<T> for AnyBody
where
    Buffered: From<T>,
{
    fn from(body: T) -> Self {
        Self::Buf(body.into())
    }
}
