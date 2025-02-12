use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::http_body::HttpBody;
use crate::error::BoxError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
const MAX_FRAME_LEN: usize = 8192; // 8KB

/// An in-memory response body that is read in `8KB` chunks.
///
#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    cursor: usize,

    /// The buffer containing the body data.
    ///
    data: Pin<Box<str>>,
}

impl ResponseBody {
    #[inline]
    pub fn new(body: &str) -> Self {
        Self {
            cursor: 0,
            data: Pin::new(body.into()),
        }
    }
}

impl Body for ResponseBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let start = self.cursor;
        let len = self.data.len();

        Poll::Ready(if start < len {
            let end = (start + MAX_FRAME_LEN).min(len);
            let data = Bytes::copy_from_slice(self.data[start..end].as_bytes());
            self.cursor = end;
            Some(Ok(Frame::data(data)))
        } else {
            None
        })
    }

    fn is_end_stream(&self) -> bool {
        self.cursor >= self.data.len()
    }

    fn size_hint(&self) -> SizeHint {
        // Get the length of the buffer and attempt to cast it to a
        // `u64`. If the cast fails, return a size hint with no bounds.
        let len = self
            .data
            .len()
            .try_into()
            .expect("failed to perform the conversion");

        SizeHint::with_exact(len)
    }
}

impl Debug for ResponseBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ResponseBody").finish()
    }
}

impl Default for ResponseBody {
    fn default() -> Self {
        Self::new("")
    }
}

impl From<String> for ResponseBody {
    fn from(string: String) -> Self {
        Self::new(&string)
    }
}

impl From<&'_ str> for ResponseBody {
    fn from(slice: &str) -> Self {
        Self::new(slice)
    }
}

impl HttpBody<ResponseBody> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for HttpBody<ResponseBody> {
    #[inline]
    fn default() -> Self {
        HttpBody::Original(Default::default())
    }
}

impl<T> From<T> for HttpBody<ResponseBody>
where
    ResponseBody: From<T>,
{
    fn from(body: T) -> Self {
        HttpBody::Original(ResponseBody::from(body))
    }
}
