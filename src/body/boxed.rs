use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

/// A struct that wraps a `dyn` Body + Send + Unpin.
#[must_use = "streams do nothing unless polled"]
pub struct NotUnpinBoxBody {
    body: Box<dyn Body<Data = Bytes, Error = Error> + Send>,
    _pin: PhantomPinned,
}

/// A struct that wraps a `dyn` Body + Send + Unpin.
#[must_use = "streams do nothing unless polled"]
pub struct UnpinBoxBody {
    body: Pin<Box<dyn Body<Data = Bytes, Error = Error> + Send + Unpin>>,
}

/// Maps the error type of a body to [Error].
///
/// This struct can only be used with [UnpinBoxBody].
///
#[must_use = "streams do nothing unless polled"]
struct MapError<B> {
    body: B,
}

impl<B: Unpin> MapError<B> {
    fn project(self: Pin<&mut Self>) -> Pin<&mut B> {
        let this = self.get_mut();
        let ptr = &mut this.body;

        Pin::new(ptr)
    }
}

impl<B, E> Body for MapError<B>
where
    B: Body<Data = Bytes, Error = E> + Send + Unpin,
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

impl NotUnpinBoxBody {
    /// Creates a new body from a dyn Body + !Unpin body.
    ///
    /// If the body is Unpin, use [UnpinBoxBody] instead.
    ///
    pub fn new<B>(body: Box<B>) -> Self
    where
        B: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        Self {
            body,
            _pin: PhantomPinned,
        }
    }
}

impl NotUnpinBoxBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut (dyn Body<Data = Bytes, Error = Error> + Send)> {
        unsafe {
            //
            // Safety:
            //
            // The `body` field is `!Unpin` because it may contain a trait object
            // that is `!Unpin`. It is the responsibility of the caller
            // `ResponseBody::pin` to ensure that the body properly remains
            // pinned while the response body is polled.
            //
            Pin::map_unchecked_mut(self, |this| &mut *this.body)
        }
    }
}

impl Body for NotUnpinBoxBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl Debug for NotUnpinBoxBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotUnpinBoxBody").finish()
    }
}

impl UnpinBoxBody {
    pub fn new<B, E>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = E> + Send + Unpin + 'static,
        Error: From<E>,
    {
        Self {
            body: Box::pin(MapError { body }),
        }
    }
}

impl Body for UnpinBoxBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::as_mut(&mut self.body).poll_frame(context)
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl Debug for UnpinBoxBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnpinBoxBody").finish()
    }
}
