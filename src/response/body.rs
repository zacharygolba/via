use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::{AnyBody, BufferedBody, NotUnpinBoxBody};
use crate::Error;

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    body: PinRequirement,
}

enum BodyProjection<'a> {
    Unpin(Pin<&'a mut AnyBody<BufferedBody>>),
    Pin(Pin<&'a mut NotUnpinBoxBody>),
}

#[derive(Debug)]
enum PinRequirement {
    Unpin(AnyBody<BufferedBody>),
    Pin(NotUnpinBoxBody),
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        let buffer = BufferedBody::new(Bytes::new());
        let body = AnyBody::Inline(buffer);

        Self {
            body: PinRequirement::Unpin(body),
        }
    }

    pub fn boxed<B, E>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = E> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let body = AnyBody::boxed(body);

        Self {
            body: PinRequirement::Unpin(body),
        }
    }
}

impl ResponseBody {
    pub(super) fn from_string(string: String) -> Self {
        let body = BufferedBody::from(string);

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(body)),
        }
    }

    pub(super) fn from_vec(bytes: Vec<u8>) -> Self {
        let body = BufferedBody::from(bytes);

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(body)),
        }
    }

    pub(super) fn try_into_unpin(self) -> Result<AnyBody<BufferedBody>, Error> {
        match self.body {
            PinRequirement::Unpin(body) => Ok(body),
            PinRequirement::Pin(_) => Err(Error::new(
                "Pinned body cannot be converted to Unpin".to_string(),
            )),
        }
    }
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> BodyProjection {
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
            PinRequirement::Unpin(ptr) => {
                // The body field is Unpin. Therefore, we can project it with
                // Pin::new without any unsafe blocks.
                BodyProjection::Unpin(Pin::new(ptr))
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

                BodyProjection::Pin(pin)
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
            BodyProjection::Unpin(unpin) => unpin.poll_frame(context),
            BodyProjection::Pin(pin) => pin.poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.body {
            PinRequirement::Unpin(unpin) => unpin.is_end_stream(),
            PinRequirement::Pin(pin) => pin.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.body {
            PinRequirement::Unpin(unpin) => unpin.size_hint(),
            PinRequirement::Pin(pin) => pin.size_hint(),
        }
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new()
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
        let buffered = BufferedBody::from(value);

        Self {
            body: PinRequirement::Unpin(AnyBody::Inline(buffered)),
        }
    }
}
