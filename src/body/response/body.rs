use bytes::{Bytes, BytesMut};
use futures_core::Stream;
use hyper::body::Frame;

use super::{Boxed, Buffered, Either, StreamAdapter, Streaming};
use crate::{Error, Result};

pub type AnyBody = Either<Either<Buffered, Streaming>, Boxed>;

pub struct ResponseBody {
    body: AnyBody,
}

impl ResponseBody {
    pub fn new() -> Self {
        Self::buffer(BytesMut::new())
    }

    pub fn buffer(data: BytesMut) -> Self {
        let buffered = Buffered::new(data);

        Self {
            body: Either::Left(Either::Left(buffered)),
        }
    }

    pub fn stream<T>(stream: T) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>>> + Send + 'static,
    {
        let stream = Streaming::new(Box::pin(stream));

        Self {
            body: Either::Left(Either::Right(stream)),
        }
    }

    pub fn stream_bytes<S, D, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<D, E>> + Send + 'static,
        D: Into<Bytes> + 'static,
        E: Into<Error> + 'static,
    {
        Self::stream(StreamAdapter::new(stream))
    }

    pub fn into_boxed(self) -> Boxed {
        match self.body {
            Either::Left(body) => Boxed::new(Box::pin(body)),
            Either::Right(boxed) => boxed,
        }
    }

    pub fn into_inner(self) -> AnyBody {
        self.body
    }

    pub fn is_empty(&self) -> bool {
        match &self.body {
            Either::Left(Either::Left(buffered)) => buffered.is_empty(),
            _ => false,
        }
    }

    pub fn len(&self) -> Option<usize> {
        match &self.body {
            Either::Left(Either::Left(buffered)) => Some(buffered.len()),
            _ => None,
        }
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new()
    }
}

impl From<()> for ResponseBody {
    fn from(_: ()) -> Self {
        Default::default()
    }
}

impl From<Boxed> for ResponseBody {
    fn from(boxed: Boxed) -> Self {
        Self {
            body: Either::Right(boxed),
        }
    }
}

impl From<Vec<u8>> for ResponseBody {
    fn from(vec: Vec<u8>) -> Self {
        let bytes = vec.as_slice();
        let buffer = BytesMut::from(bytes);

        Self::buffer(buffer)
    }
}

impl From<&'static [u8]> for ResponseBody {
    fn from(slice: &'static [u8]) -> Self {
        let buffer = BytesMut::from(slice);
        Self::buffer(buffer)
    }
}

impl From<String> for ResponseBody {
    fn from(string: String) -> Self {
        let bytes = string.as_bytes();
        let buffer = BytesMut::from(bytes);

        Self::buffer(buffer)
    }
}

impl From<&'static str> for ResponseBody {
    fn from(slice: &'static str) -> Self {
        let bytes = slice.as_bytes();
        let buffer = BytesMut::from(bytes);

        Self::buffer(buffer)
    }
}
