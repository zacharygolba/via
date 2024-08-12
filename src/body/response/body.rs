use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use hyper::body::{Body, Frame, SizeHint};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::{Buffered, Either, Mapped, Streaming};
use crate::{Error, Result};

pub struct ResponseBody {
    body: Either<Either<Buffered, Streaming>, Mapped>,
}

impl ResponseBody {
    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }

    pub fn len(&self) -> Option<usize> {
        match &self.body {
            Either::Left(Either::Left(buffered)) => Some(buffered.len()),
            Either::Right(mapped) => mapped.len(),
            _ => None,
        }
    }
}

impl ResponseBody {
    pub(crate) fn new() -> Self {
        let buffered = Buffered::empty();

        Self {
            body: Either::Left(Either::Left(buffered)),
        }
    }

    pub(crate) fn buffer(data: Bytes) -> Self {
        let buffered = Buffered::new(BytesMut::from(data));

        Self {
            body: Either::Left(Either::Left(buffered)),
        }
    }

    pub(crate) fn stream<T>(stream: T) -> Self
    where
        T: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        let stream = Streaming::new(stream);

        Self {
            body: Either::Left(Either::Right(stream)),
        }
    }

    pub(crate) fn map<F>(self, map: F) -> Self
    where
        F: Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static,
    {
        match self.body {
            Either::Left(source) => {
                let mut mapped = Mapped::new(source);

                mapped.push(map);

                Self {
                    body: Either::Right(mapped),
                }
            }
            Either::Right(mut mapped) => {
                mapped.push(map);

                Self {
                    body: Either::Right(mapped),
                }
            }
        }
    }

    /// Returns a pinned reference to the inner kind of the body.
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<Either<Buffered, Streaming>, Mapped>> {
        unsafe {
            // Safety:
            // TODO: Add safety explanation.
            let this = self.get_unchecked_mut();
            Pin::new_unchecked(&mut this.body)
        }
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl From<()> for ResponseBody {
    fn from(_: ()) -> Self {
        Self::new()
    }
}

impl From<Bytes> for ResponseBody {
    fn from(bytes: Bytes) -> Self {
        Self::buffer(bytes)
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(vec: Vec<u8>) -> Self {
        Self::buffer(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for ResponseBody {
    fn from(slice: &'static [u8]) -> Self {
        Self::buffer(Bytes::from_static(slice))
    }
}

impl From<String> for ResponseBody {
    fn from(string: String) -> Self {
        Self::buffer(Bytes::from(string))
    }
}

impl From<&'static str> for ResponseBody {
    fn from(slice: &'static str) -> Self {
        Self::buffer(Bytes::from_static(slice.as_bytes()))
    }
}
