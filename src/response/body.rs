use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::{AnyBody, BufferedBody, NotUnpinBoxBody, UnpinBoxBody};
use crate::Error;

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    body: PinRequirement,
}

#[derive(Debug)]
enum PinRequirement {
    Unpin(AnyBody<Box<BufferedBody>>),
    Pin(NotUnpinBoxBody),
}

enum ResponseBodyProjection<'a> {
    Boxed(Pin<&'a mut UnpinBoxBody>),
    Pinned(Pin<&'a mut NotUnpinBoxBody>),
    Buffer(Pin<&'a mut Box<BufferedBody>>),
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        let buffer = Box::new(BufferedBody::new(Bytes::new()));
        let body = AnyBody::Inline(buffer);

        Self {
            body: PinRequirement::Unpin(body),
        }
    }
}

impl ResponseBody {
    pub(super) fn from_boxed(body: UnpinBoxBody) -> Self {
        Self {
            body: PinRequirement::Unpin(AnyBody::Boxed(body)),
        }
    }

    pub(super) fn from_string(string: String) -> Self {
        let body = Box::new(BufferedBody::from(string));

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(body)),
        }
    }

    pub(super) fn from_vec(bytes: Vec<u8>) -> Self {
        let body = Box::new(BufferedBody::from(bytes));

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(body)),
        }
    }

    pub(super) fn try_into_unpin(self) -> Result<AnyBody<Box<BufferedBody>>, Error> {
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
                // Boxed is Unpin so we can project it without using unsafe.
                ResponseBodyProjection::Boxed(Pin::new(ptr))
            }
            PinRequirement::Unpin(AnyBody::Inline(ptr)) => {
                // Buffered is Unpin so we can project it without using unsafe.
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

impl From<UnpinBoxBody> for ResponseBody {
    fn from(boxed: UnpinBoxBody) -> Self {
        Self::from_boxed(boxed)
    }
}

impl From<NotUnpinBoxBody> for ResponseBody {
    fn from(pinned: NotUnpinBoxBody) -> Self {
        Self {
            body: PinRequirement::Pin(pinned),
        }
    }
}

impl<T> From<T> for ResponseBody
where
    BufferedBody: From<T>,
{
    fn from(value: T) -> Self {
        let buffered = Box::new(BufferedBody::from(value));

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(buffered)),
        }
    }
}
