use bytes::BytesMut;
use hyper::body::Incoming;

use super::{BodyStream, ReadIntoBytes, ReadIntoString};
use crate::Result;

/// The maximum amount of bytes that can be preallocated for a request body.
const MAX_PREALLOC_SIZE: usize = 104857600; // 100 MB

#[derive(Debug)]
pub struct RequestBody {
    body: Box<Incoming>,
    len: Option<usize>,
}

/// Preallocates a `BytesMut` buffer with the provided `capacity` if it is less
/// than or equal to `MAX_PREALLOC_SIZE`. If `capacity` is `None`, an empty
/// `BytesMut` buffer is returned.
fn bytes_mut_with_capacity(capacity: Option<usize>) -> BytesMut {
    capacity.map_or_else(BytesMut::new, |value| {
        BytesMut::with_capacity(value.min(MAX_PREALLOC_SIZE))
    })
}

impl RequestBody {
    pub fn into_stream(self) -> BodyStream {
        BodyStream::new(self.body)
    }

    pub fn read_into_bytes(self) -> ReadIntoBytes {
        let buffer = bytes_mut_with_capacity(self.len);
        let stream = self.into_stream();

        ReadIntoBytes::new(buffer, stream)
    }

    pub fn read_into_string(self) -> ReadIntoString {
        let future = self.read_into_bytes();
        ReadIntoString::new(future)
    }

    #[cfg(feature = "serde")]
    pub async fn read_json<T>(self) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        use crate::{http::StatusCode, Error};

        let buffer = self.read_into_bytes().await?;

        serde_json::from_slice(&buffer).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }
}

impl RequestBody {
    pub(crate) fn new(body: Incoming) -> Self {
        Self {
            body: Box::new(body),
            len: None,
        }
    }

    pub(crate) fn with_len(body: Incoming, len: usize) -> Self {
        Self {
            body: Box::new(body),
            len: Some(len),
        }
    }
}
