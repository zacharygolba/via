use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::Poll;

use super::{Boxed, Buffered};
use crate::Error;

/// A sum type that can represent any `Unpin` [Request](crate::Request) or
/// [Response](crate::Response) body.
#[non_exhaustive]
#[must_use = "streams do nothing unless polled"]
pub enum AnyBody<B> {
    Boxed(Boxed),
    Inline(B),
}

enum AnyBodyProjection<'a, B> {
    Boxed(Pin<&'a mut Boxed>),
    Inline(Pin<&'a mut B>),
}

impl AnyBody<Buffered> {
    pub fn new() -> Self {
        Self::Inline(Default::default())
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
    E: Into<Error>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            AnyBodyProjection::Boxed(boxed) => boxed.poll_frame(context),
            AnyBodyProjection::Inline(body) => {
                body.poll_frame(context).map_err(|error| error.into())
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Boxed(boxed) => boxed.is_end_stream(),
            Self::Inline(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Boxed(boxed) => boxed.size_hint(),
            Self::Inline(body) => body.size_hint(),
        }
    }
}

impl<B> From<Boxed> for AnyBody<B> {
    fn from(boxed: Boxed) -> Self {
        Self::Boxed(boxed)
    }
}

impl Default for AnyBody<Buffered> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for AnyBody<Buffered>
where
    Buffered: From<T>,
{
    fn from(body: T) -> Self {
        Self::Inline(body.into())
    }
}
