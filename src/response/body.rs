use bytes::Bytes;
use http_body::Body;
use std::fmt::{self, Debug, Formatter};

use crate::body::{AnyBody, BoxBody, ByteBuffer};
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
            body: AnyBody::Const(buffered),
        }
    }

    pub fn from_dyn<T, E>(body: T) -> Self
    where
        T: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        Self {
            body: AnyBody::Dyn(BoxBody::new(body)),
        }
    }
}

impl ResponseBody {
    pub(super) fn from_string(string: String) -> Self {
        let buffered = ByteBuffer::from(string);

        Self {
            body: AnyBody::Const(buffered),
        }
    }

    pub(super) fn from_vec(bytes: Vec<u8>) -> Self {
        let buffered = ByteBuffer::from(bytes);

        Self {
            body: AnyBody::Const(buffered),
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

impl From<BoxBody> for ResponseBody {
    fn from(body: BoxBody) -> Self {
        Self {
            body: AnyBody::Dyn(body),
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
            body: AnyBody::Const(buffered),
        }
    }
}
