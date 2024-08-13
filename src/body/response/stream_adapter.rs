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
pub struct StreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    /// The `Stream` that we are adapting to yield `Result<Frame<Bytes>>`.
    stream: T,

    /// This field is used to mark `StreamAdapter` as `!Unpin`. This is necessary
    /// because `T` may not be `Unpin` and we need to project the `stream` field
    /// through a pinned reference to `Self` so it can be polled.
    _pin: PhantomPinned,
}

impl<T, D, E> StreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    pub(crate) fn new(stream: T) -> Self {
        Self {
            stream,
            _pin: PhantomPinned,
        }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut T> {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // `StreamAdapter` is generic of `T` where `T` is a `Stream` that may
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
            // long as the type `T`  safely manages pinned references in it's
            // implementation since we are only using the pinned reference to
            // call the `poll_next` method.
            Pin::new_unchecked(ptr)
        }
    }
}

impl<T, D, E> Stream for StreamAdapter<T, D, E>
where
    T: Stream<Item = Result<D, E>> + Send,
    Bytes: From<D>,
    Error: From<E>,
{
    type Item = Result<Frame<Bytes>>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project()
            .poll_next(context)
            .map_ok(|data| Frame::data(Bytes::from(data)))
            .map_err(Error::from)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Get the size hint from the `stream` field.
        self.stream.size_hint()
    }
}
