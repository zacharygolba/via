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
}

impl Mapped {
    /// Returns a pinned mutable reference to the `body` field.
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<Buffered, Streaming>> {
        unsafe {
            //
            // Safety:
            //
            // All possible variants of the `Either` enum that compose the `body`
            // field are `!Unpin` and do not move data out of the pinned
            // reference from which they are polled.
            //
            self.map_unchecked_mut(|this| &mut this.body)
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
        self.as_mut().project().poll_frame(context).map(|option| {
            let frame = match option {
                // A frame was successfully polled from the stream. Store it
                // at `frame`.
                Some(Ok(value)) => value,
                // An error occurred while polling the stream. Return early.
                Some(error) => return Some(error),
                // The stream has been exhausted. Return early.
                None => return None,
            };

            // Return `Some` with the result of attempting to map the data at
            // frame.
            Some(frame.try_map_data(|input| {
                // Attempt to fold the queue of map functions over the data
                // in the frame if `frame.is_data()` and return the result.
                //
                // Due to the fact that the middleware is a stack, the map
                // functions in `queue` are applied in reverse order.
                self.queue
                    .iter()
                    .rev()
                    .try_fold(input, |data, map| map(data))
            }))
        })
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
