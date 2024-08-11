use bytes::Bytes;
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

use super::size_hint;

#[must_use = "streams do nothing unless polled"]
pub struct Map<F, B, E>
where
    F: Fn(Bytes) -> Result<Bytes, E> + Send + 'static,
    B: Body<Data = Bytes, Error = Error>,
    Error: From<E>,
{
    body: B,

    map_fn: F,

    _pinned: PhantomPinned,
}

impl<F, B, E> Map<F, B, E>
where
    F: Fn(Bytes) -> Result<Bytes, E> + Send,
    B: Body<Data = Bytes, Error = Error>,
    Error: From<E>,
{
    pub(crate) fn new(body: B, map_fn: F) -> Self {
        Self {
            body,
            map_fn,
            _pinned: PhantomPinned,
        }
    }

    fn project(self: Pin<&mut Self>) -> (Pin<&mut B>, Pin<&mut F>) {
        // Get a mutable reference to `self`.
        let this = unsafe { self.get_unchecked_mut() };
        // Get a mutable reference to the `body` field.
        let body = unsafe { Pin::new_unchecked(&mut this.body) };
        // Get a mutable reference to the `map` field.
        let map = unsafe { Pin::new_unchecked(&mut this.map_fn) };

        (body, map)
    }
}

impl<F, B, E> Body for Map<F, B, E>
where
    F: Fn(Bytes) -> Result<Bytes, E> + Send,
    B: Body<Data = Bytes, Error = Error>,
    Error: From<E>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let (body, map) = self.project();

        match body.poll_frame(context) {
            Poll::Ready(Some(Ok(frame))) if frame.is_data() => {
                let bytes = map(frame.into_data().unwrap())?;
                let frame = Frame::data(bytes);
                Poll::Ready(Some(Ok(frame)))
            }
            poll => poll,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl<F, B, E> Stream for Map<F, B, E>
where
    F: Fn(Bytes) -> Result<Bytes, E> + Send,
    B: Body<Data = Bytes, Error = Error>,
    Error: From<E>,
{
    type Item = Result<Bytes>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        size_hint::from_body_for_stream(&self.body)
    }
}
