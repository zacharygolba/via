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
    body: Either<Either<Buffered, Streaming>, Mapped>,
}

impl Pollable {
    pub fn new(body: Either<Either<Buffered, Streaming>, Mapped>) -> Self {
        Self { body }
    }
}

impl Pollable {
    /// Returns a pinned mutable reference to the `body` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<Either<Buffered, Streaming>, Mapped>> {
        unsafe {
            //
            // Safety:
            //
            // All possible variants of the nested `Either` enums that compose
            // the `body` field are `!Unpin` and do not move data out of the
            // pinned reference from which they are polled.
            //
            self.map_unchecked_mut(|this| &mut this.body)
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
        self.project().poll_frame(context)
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
