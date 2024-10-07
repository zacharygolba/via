use bytes::Bytes;
use http_body::Body;
use http_body_util::combinators::BoxBody;
use std::fmt::{self, Debug, Formatter};

use crate::body::{AnyBody, ByteBuffer};
use crate::Error;

#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    body: AnyBody<ByteBuffer>,
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        let buffered = ByteBuffer::new(&[]);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }

    pub fn from_dyn<T>(body: T) -> Self
    where
        T: Body<Data = Bytes, Error = Error> + Send + Sync + 'static,
    {
        Self {
            body: AnyBody::Box(BoxBody::new(body)),
        }
    }
}

impl ResponseBody {
    pub(super) fn from_string(string: String) -> Self {
        let buffered = ByteBuffer::from(string);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }

    pub(super) fn from_vec(bytes: Vec<u8>) -> Self {
        let buffered = ByteBuffer::from(bytes);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }

    pub(super) fn into_inner(self) -> AnyBody<ByteBuffer> {
        self.body
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

impl From<BoxBody<Bytes, Error>> for ResponseBody {
    fn from(body: BoxBody<Bytes, Error>) -> Self {
        Self {
            body: AnyBody::Box(body),
        }
    }
}

impl<T> From<T> for ResponseBody
where
    ByteBuffer: From<T>,
{
    fn from(body: T) -> Self {
        let buffered = ByteBuffer::from(body);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }
}
