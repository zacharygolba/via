use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::{Buffered, Either, Mapped, Streaming};
use crate::{Error, Result};

/// An externally immutable, pollable version of a response body.
///
/// The `ResponseBody` struct may move data out of the body field while we
/// unwind the middleware stack. This is problematic as the type would allow
/// us to potentially move data out of the body field while it is being polled
/// behind a `Pin`. To prevent this, we only implement `Body` for `Pollable` to
/// ensure that response bodies are only polled once the middleware stack has
/// been fully unwound.
pub struct Pollable {
    body: Either<Either<Buffered, Streaming>, Box<Mapped>>,
}

/// A projection type for the possible variants of the nested `Either` enums
/// that compose the `body` field of `Pollable`.
//
// This `enum` primarily exists to get a `Pin<&mut Mapped>` reference to the
// `Box<Mapped>` contained in the `Either::Right` variant of the `body` field
// of `Pollable` so we can delegate the call to `poll_frame` to `Mapped` when
// necessary since it is `!Unpin`.
enum BodyProjection<'a> {
    /// The projection type for a `Buffered` response body.
    Buffered(Pin<&'a mut Buffered>),

    /// The projection type for a `Mapped` response body.
    Mapped(Pin<&'a mut Mapped>),

    /// The projection type for a `Streaming` response body.
    Streaming(Pin<&'a mut Streaming>),
}

impl Pollable {
    pub fn new(body: Either<Either<Buffered, Streaming>, Box<Mapped>>) -> Self {
        Self { body }
    }

    /// Returns a projection type of the possible variants of the nested `Either`
    /// enums that compose the `body` field.
    fn project(self: Pin<&mut Self>) -> BodyProjection {
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
                BodyProjection::Buffered(pin)
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
                BodyProjection::Streaming(pin)
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
                BodyProjection::Mapped(pin)
            }
        }
    }
}

impl Body for Pollable {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            BodyProjection::Buffered(buffered) => buffered.poll_frame(context),
            BodyProjection::Mapped(mapped) => mapped.poll_frame(context),
            BodyProjection::Streaming(streaming) => streaming.poll_frame(context),
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
