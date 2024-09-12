use bytes::Bytes;
use http_body::Body;

use crate::body::{Buffer, EveryBody};
use crate::Error;

#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    body: EveryBody<Buffer>,
}

impl ResponseBody {
    /// Creates a new, empty response body.
    pub fn new() -> Self {
        let buffered = Buffer::new(&[]);

        Self {
            body: EveryBody::Inline(buffered),
        }
    }

    pub fn from_dyn<B, E>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        Self {
            body: EveryBody::from_dyn(body),
        }
    }
}

impl ResponseBody {
    pub(super) fn from_string(string: String) -> Self {
        let buffered = Buffer::from(string);

        Self {
            body: EveryBody::Inline(buffered),
        }
    }

    pub(super) fn from_vec(bytes: Vec<u8>) -> Self {
        let buffered = Buffer::from(bytes);

        Self {
            body: EveryBody::Inline(buffered),
        }
    }

    pub(super) fn into_inner(self) -> EveryBody<Buffer> {
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
    Buffer: From<T>,
{
    fn from(value: T) -> Self {
        let buffered = Buffer::from(value);

        Self {
            body: EveryBody::Inline(buffered),
        }
    }
}
