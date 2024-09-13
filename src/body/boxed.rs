use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

/// Converts an `impl Body + Send` into a type-erased trait object.
///
#[must_use = "streams do nothing unless polled"]
pub struct BoxBody {
    body: Pin<Box<dyn Body<Data = Bytes, Error = Error> + Send>>,
}

/// Maps the error type of a body to [Error].
///
#[must_use = "streams do nothing unless polled"]
struct MapError<T> {
    body: T,
}

impl BoxBody {
    pub fn new<T, E>(body: T) -> Self
    where
        T: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        Self {
            body: Box::pin(MapError { body }),
        }
    }
}

impl Body for BoxBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.body.as_mut().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl Debug for BoxBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxBody").finish()
    }
}

impl<T> MapError<T> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
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

impl<T, E> Body for MapError<T>
where
    T: Body<Data = Bytes, Error = E> + Send,
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
