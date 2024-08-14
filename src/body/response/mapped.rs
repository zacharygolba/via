use bytes::Bytes;
use futures_core::ready;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::{Buffered, Either, FrameExt, Streaming};
use crate::{Error, Result};

/// A type alias for the trait object that represents a map function. This type
/// alias exists to simply the type signatures in this module.
type MapFn = dyn Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static;

/// A response body that maps the data frame by frame by folding a queue of map
/// functions.
pub struct Mapped {
    body: Either<Buffered, Streaming>,
    queue: Vec<Box<MapFn>>,
}

impl Mapped {
    /// Creates a new `Mapped` response body with the given `body`.
    pub fn new(body: Either<Buffered, Streaming>) -> Self {
        Self {
            body,
            queue: Vec::new(),
        }
    }

    /// Pushes a map function into the queue of map functions.
    pub fn push<F>(&mut self, map: Box<F>)
    where
        F: Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static,
    {
        self.queue.push(map);
    }

    /// Returns `true` if `body` is buffered in memory and contains no data.
    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }

    /// Returns the byte length of the data in `body` if it is buffered in memory.
    pub fn len(&self) -> Option<usize> {
        if let Either::Left(buffered) = &self.body {
            Some(buffered.len())
        } else {
            None
        }
    }

    /// Returns a tuple that contains a pinned mutable reference to the `body`
    /// field and a shared reference to the queue of map functions.
    fn project(self: Pin<&mut Self>) -> (Pin<&mut Either<Buffered, Streaming>>, &[Box<MapFn>]) {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // The `body` field may contain a `Streaming` response body which is
            // not `Unpin`. We need to project the body field through a pinned
            // reference to `Self` so that it can be polled in the `poll_frame`
            // method. This is safe because no data is moved out of `self`.
            self.get_unchecked_mut()
        };
        // Get a pinned mutable reference to the `body` field.
        let body = unsafe {
            //
            // Safety:
            //
            // The `body` field may contain a `Streaming` response body which is
            // not `Unpin`. Therefore we have to use `Pin::new_unchecked` to wrap
            // the mutable reference to `body` in a pinned reference. This is safe
            // because we are not moving the value or any data owned by the `body`
            // field out of the pinned mutable reference.
            Pin::new_unchecked(&mut this.body)
        };

        // Return the pinned reference to the `body` field and a shared reference
        // to our queue of map functions.
        (body, &*this.queue)
    }
}

impl Body for Mapped {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let (body, queue) = self.project();

        Poll::Ready(ready!(body.poll_frame(context)).map(|result| {
            result?.try_map_data(|input| {
                // Attempt to fold the queue of map functions over the data in
                // the frame if `frame.is_data()` and return the result.
                //
                // Due to the fact that the middleware is a stack, the map
                // functions in `queue` are applied in reverse order.
                queue.iter().rev().try_fold(input, |data, map| map(data))
            })
        }))
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
