use bytes::Bytes;
use futures_core::Stream;
use hyper::body::Frame;
use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

/// A stream adapter that converts a stream of `Result<D, E>` into a stream of
/// `Result<Frame<Bytes>>`. This adapter allows for response bodies to be built
/// from virtually any stream that yields data that can be converted into bytes.
#[must_use = "streams do nothing unless polled"]
pub struct StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    /// The `Stream` that we are adapting to yield `Result<Frame<Bytes>>`.
    stream: S,

    /// This field is used to mark `StreamAdapter` as `!Unpin`. This is necessary
    /// because `S` may not be `Unpin` and we need to project the `stream` field
    /// through a pinned reference to `Self` so it can be polled.
    _pin: PhantomPinned,
}

impl<S, D, E> StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream,
            _pin: PhantomPinned,
        }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut S> {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // `StreamAdapter` is generic of `S` where `S` is a `Stream` that may
            // or may not be `!Unpin`. Due to this, we include a `_pin` field in
            // `StreamAdapter` to mark the struct as `!Unpin`. In order to get a
            // mutable reference to `self` from the pinned reference, we need to
            // use `Pin::new_unchecked`. This is safe as long as the `stream`
            // field that we are projecting safely manages pinned references in
            // it's implementation.
            self.get_unchecked_mut()
        };
        // Get a mutable reference to the `stream` field.
        let ptr = &mut this.stream;

        // Return the pinned reference to the `stream` field.
        unsafe {
            //
            // Safety:
            //
            // The `stream` field may or not be `!Unpin`. We have to use
            // `Pin::new_unchecked` to pin the reference `ptr`. This is safe as
            // long as the type `S` safely manages pinned references in it's
            // implementation since we are only using the pinned reference to
            // call the `poll_next` method.
            Pin::new_unchecked(ptr)
        }
    }
}

impl<S, D, E> Stream for StreamAdapter<S, D, E>
where
    S: Stream<Item = Result<D, E>> + Send,
    D: Into<Bytes>,
    E: Into<Error>,
{
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .poll_next(context)
            .map_ok(|data| Frame::data(data.into()))
            .map_err(|error| error.into())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Get the size hint from the `stream` field.
        self.stream.size_hint()
    }
}
