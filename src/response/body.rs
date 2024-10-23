use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::Either;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::{BoxError, Error};

pub struct ResponseBody {
    kind: Either<String, BoxBody<Bytes, BoxError>>,
}

impl ResponseBody {
    /// Creates a new response body.
    ///
    #[inline]
    pub fn new(kind: Either<String, BoxBody<Bytes, BoxError>>) -> Self {
        Self { kind }
    }
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<String, BoxBody<Bytes, BoxError>>> {
        let this = self.get_mut();
        let ptr = &mut this.kind;

        Pin::new(ptr)
    }
}

impl Debug for ResponseBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.kind, f)
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new(Either::Left(String::new()))
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.kind.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.kind.size_hint()
    }
}

impl From<String> for ResponseBody {
    #[inline]
    fn from(string: String) -> Self {
        Self {
            kind: Either::Left(string),
        }
    }
}

impl From<BoxBody<Bytes, BoxError>> for ResponseBody {
    #[inline]
    fn from(body: BoxBody<Bytes, BoxError>) -> Self {
        Self {
            kind: Either::Right(body),
        }
    }
}

impl TryFrom<Vec<u8>> for ResponseBody {
    type Error = Error;

    #[inline]
    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(String::from_utf8(vec)?.into())
    }
}
