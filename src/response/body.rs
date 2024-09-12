use bytes::Bytes;
use http_body::Body;

use crate::body::{AnyBody, BufferedBody};
use crate::Error;

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    body: AnyBody<BufferedBody>,
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        let buffered = BufferedBody::new(&[]);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }

    pub fn boxed<B, E>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        Self {
            body: AnyBody::boxed(body),
        }
    }
}

impl ResponseBody {
    pub(super) fn from_string(string: String) -> Self {
        let buffered = BufferedBody::from(string);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }

    pub(super) fn from_vec(bytes: Vec<u8>) -> Self {
        let buffered = BufferedBody::from(bytes);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }

    pub(super) fn into_inner(self) -> AnyBody<BufferedBody> {
        self.body
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for ResponseBody
where
    BufferedBody: From<T>,
{
    fn from(value: T) -> Self {
        let buffered = BufferedBody::from(value);

        Self {
            body: AnyBody::Inline(buffered),
        }
    }
}
