use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::{AnyBody, Boxed, Buffer, Pinned};
use crate::Error;

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    body: PinRequirement,
}

#[derive(Debug)]
enum PinRequirement {
    Unpin(AnyBody<Buffer>),
    Pin(Pinned),
}

enum ResponseBodyProjection<'a> {
    Boxed(Pin<&'a mut Boxed>),
    Pinned(Pin<&'a mut Pinned>),
    Buffer(Pin<&'a mut Buffer>),
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        Self {
            body: PinRequirement::Unpin(AnyBody::new()),
        }
    }
}

impl ResponseBody {
    pub(crate) fn try_into_unpin(self) -> Result<AnyBody<Buffer>, Error> {
        match self.body {
            PinRequirement::Unpin(body) => Ok(body),
            PinRequirement::Pin(_) => Err(Error::new(
                "Pinned body cannot be converted to Unpin".to_string(),
            )),
        }
    }
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> ResponseBodyProjection {
        let this = unsafe {
            //
            // Safety:
            //
            // The body field may contain a Pinned body, which is !Unpin. We are
            // not moving the value out of `self`, so it is safe to project the
            // field.
            //
            self.get_unchecked_mut()
        };

        match &mut this.body {
            PinRequirement::Unpin(AnyBody::Boxed(ptr)) => {
                // Boxed is Unpin so we can project it with using unsafe.
                ResponseBodyProjection::Boxed(Pin::new(ptr))
            }
            PinRequirement::Unpin(AnyBody::Inline(ptr)) => {
                // Buffer is Unpin so we can project it with using unsafe.
                ResponseBodyProjection::Buffer(Pin::new(ptr))
            }
            PinRequirement::Pin(ptr) => {
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // Pinned is `!Unpin` because it was created from an impl Body
                    // that is !Unpin. We are not moving the value out of `self`,
                    // so it is safe to project the field.
                    //
                    Pin::new_unchecked(ptr)
                };

                ResponseBodyProjection::Pinned(pin)
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
            ResponseBodyProjection::Boxed(boxed) => boxed.poll_frame(context),
            ResponseBodyProjection::Pinned(pinned) => pinned.poll_frame(context),
            ResponseBodyProjection::Buffer(buffered) => buffered.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.body {
            PinRequirement::Unpin(body) => body.is_end_stream(),
            PinRequirement::Pin(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.body {
            PinRequirement::Unpin(body) => body.size_hint(),
            PinRequirement::Pin(body) => body.size_hint(),
        }
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Boxed> for ResponseBody {
    fn from(boxed: Boxed) -> Self {
        Self {
            body: PinRequirement::Unpin(AnyBody::Boxed(boxed)),
        }
    }
}

impl From<Pinned> for ResponseBody {
    fn from(pinned: Pinned) -> Self {
        Self {
            body: PinRequirement::Pin(pinned),
        }
    }
}

impl<T> From<T> for ResponseBody
where
    Buffer: From<T>,
{
    fn from(value: T) -> Self {
        let buffered = Buffer::from(value);

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(buffered)),
        }
    }
}
