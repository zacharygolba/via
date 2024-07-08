use futures_util::Stream;
use http_body::{Body as HttpBody, SizeHint};
use http_body_util::{BodyExt, Empty, Full, StreamBody};
use hyper::body::Bytes;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use crate::{Error, Result};

pub type Frame = http_body::Frame<Bytes>;

type BodyData = dyn http_body::Body<Data = Bytes, Error = Error> + Send + 'static;
type BoxStream = Pin<Box<dyn Stream<Item = Result<Frame>> + Send + 'static>>;

pub struct Body {
    data: Box<BodyData>,
    len: Option<usize>,
}

impl Body {
    pub fn len(&self) -> Option<usize> {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len.map_or(true, |len| len == 0)
    }
}

impl Body {
    pub(super) fn empty() -> Self {
        Self {
            data: Box::new(Empty::new().map_err(Error::from)),
            len: Some(0),
        }
    }

    pub(super) fn full(body: Bytes) -> Self {
        let len = body.len();

        Self {
            data: Box::new(Full::new(body).map_err(Error::from)),
            len: Some(len),
        }
    }

    pub(super) fn stream(body: BoxStream) -> Self {
        Self {
            data: Box::new(StreamBody::new(body)),
            len: None,
        }
    }

    fn project(self: Pin<&mut Self>) -> Pin<&mut BodyData> {
        // SAFETY:
        // A pin projection.
        unsafe {
            let this = self.get_unchecked_mut();
            Pin::new_unchecked(&mut *this.data)
        }
    }
}

impl From<()> for Body {
    fn from(_: ()) -> Self {
        Self::empty()
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Self::full(bytes)
    }
}

impl From<Vec<u8>> for Body {
    fn from(vec: Vec<u8>) -> Self {
        Self::full(Bytes::from(vec))
    }
}

impl From<&'static [u8]> for Body {
    fn from(slice: &'static [u8]) -> Self {
        Self::full(Bytes::from_static(slice))
    }
}

impl From<String> for Body {
    fn from(string: String) -> Self {
        Self::full(Bytes::from(string))
    }
}

impl From<&'static str> for Body {
    fn from(slice: &'static str) -> Self {
        Self::full(Bytes::from_static(slice.as_bytes()))
    }
}

impl HttpBody for Body {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.data.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.data.size_hint()
    }
}
