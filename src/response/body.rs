use bytes::Bytes;
use http_body::{Body, Frame};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::Either;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

pub type DynBody = UnsyncBoxBody<Bytes, Error>;

pub struct ResponseBody {
    body: Either<DynBody, String>,
}

impl ResponseBody {
    /// Creates a new, empty response body.
    ///
    #[inline]
    pub fn new() -> Self {
        Self {
            body: Either::Right(String::new()),
        }
    }

    /// Creates a new, empty response body.
    ///
    #[inline]
    pub fn from_dyn<T>(body: T) -> Self
    where
        T: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        Self {
            body: Either::Left(DynBody::new(body)),
        }
    }
}

impl ResponseBody {
    pub(super) fn into_inner(self) -> Either<DynBody, String> {
        self.body
    }
}

impl ResponseBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<DynBody, String>> {
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
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project()
            .poll_frame(context)
            .map_err(Error::from_box_error)
    }
}

impl From<DynBody> for ResponseBody {
    #[inline]
    fn from(body: DynBody) -> Self {
        Self {
            body: Either::Left(body),
        }
    }
}

impl From<String> for ResponseBody {
    #[inline]
    fn from(string: String) -> Self {
        Self {
            body: Either::Right(string),
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
