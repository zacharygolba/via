use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BufferedBody;
use crate::Error;

/// A sum type that can represent any
/// [Request](crate::Request)
/// or
/// [Response](crate::Response)
/// body.
///
#[must_use = "streams do nothing unless polled"]
pub enum AnyBody<B> {
    Dyn(Pin<Box<dyn Body<Data = Bytes, Error = Error> + Send>>),
    Inline(B),
}

enum AnyBodyProjection<'a, B> {
    Dyn(Pin<&'a mut (dyn Body<Data = Bytes, Error = Error> + Send)>),
    Inline(Pin<&'a mut B>),
}

/// Maps the error type of a body to [Error].
///
#[must_use = "streams do nothing unless polled"]
struct MapError<B> {
    body: B,
}

impl AnyBody<BufferedBody> {
    pub fn new() -> Self {
        Self::Inline(Default::default())
    }
}

impl<B> AnyBody<B> {
    pub fn boxed<U, E>(body: U) -> Self
    where
        U: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        Self::Dyn(Box::pin(MapError { body }))
    }
}

impl<B> AnyBody<B> {
    fn project(self: Pin<&mut Self>) -> AnyBodyProjection<B> {
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
            // The Dyn variant is always pinned. We are simply reborrowing the
            // pinned reference as we would in the body of poll_frame if this
            // type simply wrapped a Pin<Box<dyn Body + Send>>.
            //
            Self::Dyn(pin) => AnyBodyProjection::Dyn(Pin::as_mut(pin)),

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

impl<B, E> Body for AnyBody<B>
where
    B: Body<Data = Bytes, Error = E>,
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

impl<B: Debug> Debug for AnyBody<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dyn(_) => f.debug_struct("BoxBody").finish(),
            Self::Inline(body) => Debug::fmt(body, f),
        }
    }
}

impl Default for AnyBody<BufferedBody> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B> MapError<B> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut B> {
        //
        // Safety:
        //
        // The body field may contain a type that is !Unpin. We need a pinned
        // reference to the body field in order to call poll_frame. This is
        // safe because the body field is never moved out of self.
        //
        unsafe { Pin::map_unchecked_mut(self, |this| &mut this.body) }
    }
}

impl<B, E> Body for MapError<B>
where
    B: Body<Data = Bytes, Error = E> + Send,
    Error: From<E>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project()
            .poll_frame(context)
            .map_err(|error| error.into())
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
