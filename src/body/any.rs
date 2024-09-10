use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::BufferedBody;
use crate::Error;

/// A sum type that can represent any `Unpin` [Request](crate::Request) or
/// [Response](crate::Response) body.
///
#[must_use = "streams do nothing unless polled"]
pub enum AnyBody<B> {
    Boxed(Pin<Box<dyn Body<Data = Bytes, Error = Error> + Send + Unpin>>),
    Inline(B),
}

enum AnyBodyProjection<'a, B> {
    Boxed(Pin<&'a mut (dyn Body<Data = Bytes, Error = Error> + Send + Unpin)>),
    Inline(Pin<&'a mut B>),
}

/// Maps the error type of a body to [Error].
///
/// This struct can only be used with [UnpinBoxBody].
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

impl<I> AnyBody<I> {
    pub fn boxed<B, E>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = E> + Send + Unpin + 'static,
        Error: From<E>,
    {
        Self::Boxed(Box::pin(MapError { body }))
    }
}

impl<B: Unpin> AnyBody<B> {
    fn project(self: Pin<&mut Self>) -> AnyBodyProjection<B> {
        match self.get_mut() {
            Self::Boxed(ptr) => AnyBodyProjection::Boxed(Pin::new(ptr)),
            Self::Inline(ptr) => AnyBodyProjection::Inline(Pin::new(ptr)),
        }
    }
}

impl<B, E> Body for AnyBody<B>
where
    B: Body<Data = Bytes, Error = E> + Unpin,
    Error: From<E>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            AnyBodyProjection::Boxed(boxed) => boxed.poll_frame(context),
            AnyBodyProjection::Inline(inline) => {
                inline.poll_frame(context).map_err(|error| error.into())
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Boxed(boxed) => boxed.is_end_stream(),
            Self::Inline(inline) => inline.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Boxed(boxed) => boxed.size_hint(),
            Self::Inline(inline) => inline.size_hint(),
        }
    }
}

impl<B: Debug> Debug for AnyBody<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Boxed(_) => f.debug_struct("BoxBody").finish(),
            Self::Inline(inline) => Debug::fmt(inline, f),
        }
    }
}

impl Default for AnyBody<BufferedBody> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for AnyBody<BufferedBody>
where
    BufferedBody: From<T>,
{
    fn from(body: T) -> Self {
        Self::Inline(body.into())
    }
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
