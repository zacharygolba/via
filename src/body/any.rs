use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::Poll;

use super::{Boxed, Buffered};
use crate::Error;

/// A sum type that representing any type of body.
#[non_exhaustive]
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
        let buffered = Buffered::default();
        AnyBody::Buf(buffered)
    }
}

impl AnyBody {
    fn project(self: Pin<&mut Self>) -> AnyBodyProjection {
        match self.get_mut() {
            AnyBody::Dyn(ptr) => {
                let pin = Pin::new(ptr);
                AnyBodyProjection::Dyn(pin)
            }
            AnyBody::Buf(ptr) => {
                let pin = Pin::new(ptr);
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
            AnyBody::Dyn(boxed) => boxed.is_end_stream(),
            AnyBody::Buf(buffered) => buffered.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            AnyBody::Dyn(boxed) => boxed.size_hint(),
            AnyBody::Buf(buffered) => buffered.size_hint(),
        }
    }
}

impl Default for AnyBody {
    fn default() -> Self {
        AnyBody::new()
    }
}

impl From<Boxed> for AnyBody {
    fn from(boxed: Boxed) -> Self {
        AnyBody::Dyn(boxed)
    }
}

impl<T> From<T> for AnyBody
where
    Buffered: From<T>,
{
    fn from(body: T) -> Self {
        AnyBody::Buf(body.into())
    }
}
