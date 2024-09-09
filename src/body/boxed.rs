use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

/// A struct that wraps a `Pin<Box<dyn Body>>`.
#[must_use = "streams do nothing unless polled"]
pub struct Boxed {
    body: Pin<Box<dyn Body<Data = Bytes, Error = Error> + Send>>,
}

/// Maps the error type of a body to [Error].
#[must_use = "streams do nothing unless polled"]
struct MapError<B> {
    body: B,
}

impl Boxed {
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

impl Body for Boxed {
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
