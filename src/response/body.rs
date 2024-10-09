use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::Either;
use std::error::Error as StdError;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

type DynBody = UnsyncBoxBody<Bytes, Box<dyn StdError + Send + Sync>>;

pub struct ResponseBody {
    body: Either<String, DynBody>,
}

impl ResponseBody {
    /// Creates a new, empty response body.
    ///
    #[inline]
    pub fn new() -> Self {
        Self {
            body: Either::Left(String::new()),
        }
    }

    /// Creates a new, empty response body.
    ///
    #[inline]
    pub fn from_dyn<B>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = Box<dyn StdError + Send + Sync>> + Send + 'static,
    {
        Self {
            body: Either::Right(DynBody::new(body)),
        }
    }
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<String, DynBody>> {
        let this = self.get_mut();
        let ptr = &mut this.body;
        Pin::new(ptr)
    }
}

impl Debug for ResponseBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.body, f)
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new()
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = Box<dyn StdError + Send + Sync>;

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

impl From<String> for ResponseBody {
    #[inline]
    fn from(string: String) -> Self {
        Self {
            body: Either::Left(string),
        }
    }
}

impl From<DynBody> for ResponseBody {
    #[inline]
    fn from(body: DynBody) -> Self {
        Self {
            body: Either::Right(body),
        }
    }
}

impl TryFrom<Vec<u8>> for ResponseBody {
    type Error = Error;

    #[inline]
    fn try_from(utf8: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(String::from_utf8(utf8)?.into())
    }
}
