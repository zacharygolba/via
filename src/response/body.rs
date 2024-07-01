use bytes::Bytes;
use futures_util::Stream;
use http_body::{Body as HttpBody, SizeHint};
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

pub type Frame = http_body::Frame<Bytes>;

type BoxStream = Pin<Box<dyn Stream<Item = Result<Frame>> + Send + 'static>>;

pub struct Body {
    data: Pin<Box<dyn HttpBody<Data = Bytes, Error = Error> + Send + 'static>>,
    len: Option<usize>,
}

impl Body {
    pub fn len(&self) -> Option<usize> {
        self.len
    }
}

impl Body {
    pub(super) fn empty() -> Self {
        Self {
            data: Box::pin(Empty::new().map_err(Error::from)),
            len: Some(0),
        }
    }

    pub(super) fn full(body: Bytes) -> Self {
        let len = body.len();

        Self {
            data: Box::pin(Full::new(body).map_err(Error::from)),
            len: Some(len),
        }
    }

    pub(super) fn stream(body: BoxStream) -> Self {
        Self {
            data: Box::pin(StreamBody::new(body)),
            len: None,
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
        Body::full(bytes)
    }
}

impl From<Vec<u8>> for Body {
    fn from(vec: Vec<u8>) -> Self {
        Body::full(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for Body {
    fn from(slice: &'static [u8]) -> Self {
        Body::full(Bytes::from_static(slice))
    }
}

impl From<String> for Body {
    fn from(string: String) -> Self {
        Body::full(Bytes::from(string))
    }
}

impl From<&'static str> for Body {
    fn from(slice: &'static str) -> Self {
        Body::full(Bytes::from_static(slice.as_bytes()))
    }
}

impl HttpBody for Body {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame, Self::Error>>> {
        let this = self.get_mut();
        this.data.as_mut().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.data.size_hint()
    }
}
