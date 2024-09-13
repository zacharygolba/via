use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BoxBody;
use crate::Error;

/// A sum type that can represent any
/// [Request](crate::Request)
/// or
/// [Response](crate::Response)
/// body.
///
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub enum AnyBody<T> {
    Dyn(BoxBody),
    Inline(T),
}

enum AnyBodyProjection<'a, T> {
    Dyn(Pin<&'a mut BoxBody>),
    Inline(Pin<&'a mut T>),
}

impl<T> AnyBody<T> {
    fn project(self: Pin<&mut Self>) -> AnyBodyProjection<T> {
        //
        // Safety:
        //
        // We need to match self in order to project the pinned reference for
        // the respective variant. This is safe because we do not move the
        // value out of self in any of the subsequent match arms.
        //
        match unsafe { self.get_unchecked_mut() } {
            //
            // Safety:
            //
            // The Dyn variant contains a BoxBody which wraps a trait object.
            // The compiler ensures that BoxBody always remains pinned since
            // the trait object is wrapped in a Pin<Box<...>>. We also do not
            // move the value out of self, so it is safe to project it.
            //
            Self::Dyn(ptr) => AnyBodyProjection::Dyn(Pin::new(ptr)),

            //
            // Safety:
            //
            // The Inline variant may contain a type that is !Unpin. We are not
            // moving the value out of self, so it is safe to project it.
            //
            Self::Inline(ptr) => AnyBodyProjection::Inline(unsafe { Pin::new_unchecked(ptr) }),
        }
    }
}

impl<T, E> Body for AnyBody<T>
where
    T: Body<Data = Bytes, Error = E>,
    Error: From<E>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            AnyBodyProjection::Dyn(body) => body.poll_frame(context),
            AnyBodyProjection::Inline(body) => {
                body.poll_frame(context).map_err(|error| error.into())
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Dyn(body) => body.is_end_stream(),
            Self::Inline(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Dyn(body) => body.size_hint(),
            Self::Inline(body) => body.size_hint(),
        }
    }
}
