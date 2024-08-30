use bytes::Bytes;
use hyper::body::{Body, Frame, SizeHint};
use smallvec::SmallVec;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{Buffered, Either, FrameExt, Streaming};
use crate::{Error, Result};

/// A type alias for the trait object that represents a map function. This type
/// alias exists to simply the type signatures in this module.
type MapFn = dyn Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static;

/// A response body that maps the data frame by frame by folding a queue of map
/// functions.
pub struct Mapped {
    /// The response body that will be used as the source of frames to map in
    /// the implementation of `Body::poll_frame`.
    body: Either<Buffered, Streaming>,

    /// A queue of map functions that are applied to data frames as they are
    /// polled from `body` in the implementation of `Body::poll_frame`.
    queue: SmallVec<[Box<MapFn>; 1]>,
}

impl Mapped {
    /// Creates a new `Mapped` response body with the given `body`.
    pub fn new(body: Either<Buffered, Streaming>) -> Self {
        Self {
            body,
            queue: SmallVec::new(),
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
        if let Either::Left(buffered) = &self.body {
            buffered.is_empty()
        } else {
            false
        }
    }

    /// Returns the byte length of the data in `body` if it is buffered in memory.
    pub fn len(&self) -> Option<usize> {
        if let Either::Left(buffered) = &self.body {
            Some(buffered.len())
        } else {
            None
        }
    }
}

impl Mapped {
    /// Returns a pinned mutable reference to the `body` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<Buffered, Streaming>> {
        // Get a mutable reference to `self`.
        let this = unsafe {
            //
            // Safety:
            //
            // The `body` field may contain a `Streaming` response body which is
            // not `Unpin`. We need to project the body field through a pinned
            // reference to `Self` so that it can be polled in the `poll_frame`
            // method. This is safe because no data is moved out of `self`.
            //
            self.get_unchecked_mut()
        };
        // Get a mutable reference to the `body` field.
        let ptr = &mut this.body;

        // Return the pinned mutable reference to the `Body` at `ptr`.
        unsafe {
            //
            // Safety:
            //
            // The `body` field may contain a `Streaming` response body which is
            // not `Unpin`. Therefore we have to use `Pin::new_unchecked` to wrap
            // the mutable reference to `body` in a pinned reference. This is safe
            // because we are not moving the value or any data owned by the `body`
            // field out of the pinned mutable reference.
            //
            Pin::new_unchecked(ptr)
        }
    }
}

impl Body for Mapped {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        // Poll `self.body` for the next frame.
        let poll = self.as_mut().project().poll_frame(context);
        // Store a reference to our queue of map functions at `queue`.
        let queue = &self.queue;

        match poll {
            // A frame was successfully polled from `self.body`. Apply our map
            // functions to the frame if it is a data frame and return the
            // result.
            Poll::Ready(Some(Ok(frame))) => {
                let result = frame.try_map_data(|input| {
                    // Attempt to fold the queue of map functions over the data
                    // in the frame if `frame.is_data()` and return the result.
                    //
                    // Due to the fact that the middleware is a stack, the map
                    // functions in `queue` are applied in reverse order.
                    queue.iter().rev().try_fold(input, |data, map| map(data))
                });

                Poll::Ready(Some(result))
            }
            // An error occurred while polling the stream. Return `Ready`
            // with the error.
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
            // The stream has been exhausted.
            Poll::Ready(None) => Poll::Ready(None),
            // Wait for the next frame.
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        // Delegate the call to `is_end_stream` to the `Body` at `self.body`.
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        // Delegate the call to `size_hint` to the `Body` at `self.body`.
        self.body.size_hint()
    }
}
