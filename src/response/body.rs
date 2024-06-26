use futures::Stream;
use http_body_util::{Full, StreamBody};
use hyper::body::{Body as HyperBody, Bytes, SizeHint};
use std::{
    pin::Pin,
    task::{self, Poll},
};

use crate::{Error, Result};

type BoxStream = Pin<Box<dyn Stream<Item = Result<Frame>> + Send + 'static>>;
type Frame = hyper::body::Frame<Bytes>;

pub struct Body {
    kind: BodyKind,
}

enum BodyKind {
    Empty,
    Full(Full<Bytes>),
    #[cfg_attr(not(feature = "file-system"), allow(dead_code))]
    Stream(StreamBody<BoxStream>),
}

impl Body {
    pub(super) fn empty() -> Self {
        Body {
            kind: BodyKind::Empty,
        }
    }

    pub(super) fn full(body: Full<Bytes>) -> Self {
        Body {
            kind: BodyKind::Full(body),
        }
    }

    #[cfg_attr(not(feature = "file-system"), allow(dead_code))]
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

#[cfg(feature = "file-system")]
impl From<tokio::fs::File> for Body {
    fn from(file: tokio::fs::File) -> Self {
        use futures::stream::StreamExt;
        use tokio_util::io::ReaderStream;

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
            BodyKind::Empty => Poll::Ready(None),
            BodyKind::Full(ref mut full) => Pin::new(full).poll_frame(context).map_err(Error::from),
            BodyKind::Stream(ref mut stream) => Pin::new(stream).poll_frame(context),
        }
    }

    fn is_end_stream(&self) -> bool {
        match &self.kind {
            BodyKind::Empty => true,
            BodyKind::Full(full) => HyperBody::is_end_stream(full),
            BodyKind::Stream(stream) => HyperBody::is_end_stream(stream),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match &self.kind {
            BodyKind::Empty => SizeHint::with_exact(0),
            BodyKind::Full(full) => HyperBody::size_hint(full),
            BodyKind::Stream(stream) => HyperBody::size_hint(stream),
        }
    }
}
