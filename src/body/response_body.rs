use bytes::Bytes;
use http_body::{Body, Frame, SizeHint};
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use super::http_body::HttpBody;
use super::limit_error::LimitError;
use crate::error::BoxError;

/// The maximum amount of data that can be read from a buffered body per frame.
///
const MAX_FRAME_LEN: usize = 8192; // 8KB

#[must_use = "streams do nothing unless polled"]
pub struct ResponseBody {
    buf: Bytes,
    cursor: usize,
}

impl ResponseBody {
    #[inline]
    pub fn new(data: &[u8]) -> Self {
        Self::from_raw(Bytes::copy_from_slice(data))
    }

    #[inline]
    pub fn from_raw(data: Bytes) -> Self {
        Self {
            buf: data,
            cursor: 0,
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
        let len = self.buf.len();

        // Determine if there is any more data to be read out of buf.
        if start >= len {
            return Poll::Ready(None);
        }

        // Calculate the byte position of the last byte of the next data frame.
        // If an overflow occurs, return an error.
        let end = start
            .checked_add(MAX_FRAME_LEN.min(len))
            .ok_or_else(|| Box::new(LimitError))?;

        self.cursor = end;

        Poll::Ready(Some(Ok(Frame::data(self.buf.slice(start..end)))))
    }

    fn is_end_stream(&self) -> bool {
        self.cursor >= self.buf.len()
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(self.buf.len().try_into().unwrap())
    }
}

impl Debug for ResponseBody {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("ResponseBody").finish()
    }
}

impl Default for ResponseBody {
    #[inline]
    fn default() -> Self {
        Self::from_raw(Bytes::new())
    }
}

impl From<String> for ResponseBody {
    #[inline]
    fn from(data: String) -> Self {
        Self::new(data.as_bytes())
    }
}

impl From<&'_ str> for ResponseBody {
    #[inline]
    fn from(data: &str) -> Self {
        Self::new(data.as_bytes())
    }
}

impl From<Vec<u8>> for ResponseBody {
    #[inline]
    fn from(data: Vec<u8>) -> Self {
        Self::new(&data)
    }
}

impl From<&'_ [u8]> for ResponseBody {
    #[inline]
    fn from(data: &'_ [u8]) -> Self {
        Self::new(data)
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
