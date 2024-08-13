use bytes::{Bytes, BytesMut};
use futures_core::Stream;

use super::{Buffered, Either, Mapped, Pollable, Streaming};
use crate::Result;

pub struct ResponseBody {
    body: Either<Either<Buffered, Streaming>, Box<Mapped>>,
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
        Self::buffer(Bytes::new())
    }

    pub(crate) fn buffer(data: Bytes) -> Self {
        let buffer = Box::new(BytesMut::from(data));
        let buffered = Buffered::new(buffer);

        Self {
            body: Either::Left(Either::Left(buffered)),
        }
    }

    pub(crate) fn stream<T>(stream: T) -> Self
    where
        T: Stream<Item = Result<Bytes>> + Send + 'static,
    {
        let stream = Streaming::new(Box::new(stream));

        Self {
            body: Either::Left(Either::Right(stream)),
        }
    }

    pub(crate) fn into_pollable(self) -> Pollable {
        Pollable::new(self.body)
    }

    pub(crate) fn map<F>(self, map: Box<F>) -> Self
    where
        F: Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static,
    {
        match self.body {
            Either::Left(source) => {
                let mut mapped = Mapped::new(source);

                mapped.push(map);

                Self {
                    body: Either::Right(Box::new(mapped)),
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
