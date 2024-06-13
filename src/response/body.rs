use futures::stream::StreamExt;
use http_body_util::{Full, StreamBody};
use hyper::body::{Body as HyperBody, Bytes};
use std::{
    pin::Pin,
    task::{self, Poll},
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{Error, Result};

pub(super) type BoxStream = futures::stream::BoxStream<'static, Result<Frame>>;
pub(super) type Frame = hyper::body::Frame<Bytes>;

pub struct Body {
    kind: BodyKind,
}

enum BodyKind {
    Full(Full<Bytes>),
    Stream(StreamBody<BoxStream>),
}

impl Body {
    pub(super) fn empty() -> Self {
        Bytes::new().into()
    }

    pub(super) fn full(body: Full<Bytes>) -> Self {
        Body {
            kind: BodyKind::Full(body),
        }
    }

    pub(super) fn stream(body: StreamBody<BoxStream>) -> Self {
        Body {
            kind: BodyKind::Stream(body),
        }
    }
}

impl From<()> for Body {
    fn from(_: ()) -> Self {
        Body::empty()
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Body::full(bytes.into())
    }
}

impl From<Vec<u8>> for Body {
    fn from(vec: Vec<u8>) -> Self {
        Body::full(vec.into())
    }
}

impl From<&'static [u8]> for Body {
    fn from(slice: &'static [u8]) -> Self {
        Body::full(slice.into())
    }
}

impl From<File> for Body {
    fn from(file: File) -> Self {
        Body::stream(StreamBody::new(
            ReaderStream::new(file)
                .map(|result| result.map(Frame::data).map_err(Error::from))
                .boxed(),
        ))
    }
}

impl From<String> for Body {
    fn from(string: String) -> Self {
        Body::full(string.into())
    }
}

impl From<&'static str> for Body {
    fn from(slice: &'static str) -> Self {
        Body::full(slice.into())
    }
}

impl HyperBody for Body {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut task::Context<'_>,
    ) -> Poll<Option<Result<Frame, Self::Error>>> {
        match self.get_mut().kind {
            BodyKind::Full(ref mut full) => Pin::new(full).poll_frame(context).map_err(Error::from),
            BodyKind::Stream(ref mut stream) => Pin::new(stream).poll_frame(context),
        }
    }
}
