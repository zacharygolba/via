use bytes::Bytes;
use futures_core::ready;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::{Buffered, Streaming};
use crate::{
    body::{frame::FrameExt, Either},
    Error, Result,
};

pub struct Mapped {
    body: Either<Buffered, Streaming>,
    queue: Vec<Box<dyn Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static>>,
}

impl Mapped {
    pub fn new(body: Either<Buffered, Streaming>) -> Self {
        Self {
            body,
            queue: Vec::new(),
        }
    }

    pub fn push<F>(&mut self, map: Box<F>)
    where
        F: Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static,
    {
        self.queue.push(map);
    }

    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }

    pub fn len(&self) -> Option<usize> {
        if let Either::Left(buffered) = &self.body {
            Some(buffered.len())
        } else {
            None
        }
    }

    #[allow(clippy::type_complexity)]
    fn project(
        self: Pin<&mut Self>,
    ) -> (
        Pin<&mut Either<Buffered, Streaming>>,
        Pin<&[Box<dyn Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static>]>,
    ) {
        // Get a mutable reference to `self`.
        let this = unsafe {
            // Safety:
            // TODO: Add safety explanation.
            self.get_unchecked_mut()
        };
        // Get a mutable reference to the `body` field.
        let source = unsafe {
            // Safety:
            // TODO: Add safety explanation.
            let ptr = &mut this.body;
            Pin::new_unchecked(ptr)
        };
        // Get a shared pinned reference to the `queue` field.
        let queue = Pin::new(&*this.queue);

        (source, queue)
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
        let frame = ready!(body.poll_frame(context)).map(|result| {
            result?.try_map_data(|input| {
                // Attempt to fold the queue of map functions over the data in
                // the frame if `frame.is_data()` and return the result.
                queue.iter().try_fold(input, |data, map| map(data))
            })
        });

        Poll::Ready(frame)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}
