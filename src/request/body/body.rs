use http::StatusCode;
use hyper::body::Incoming;
use serde::de::DeserializeOwned;

use super::{BodyStream, ReadIntoBytes, ReadIntoString};
use crate::body::EveryBody;
use crate::Error;

#[derive(Debug)]
pub struct RequestBody {
    body: EveryBody<Incoming>,
}

impl RequestBody {
    pub fn into_inner(self) -> EveryBody<Incoming> {
        self.body
    }

    pub fn into_stream(self) -> BodyStream {
        BodyStream::new(self.body)
    }

    pub fn read_into_bytes(self) -> ReadIntoBytes {
        let buffer = Vec::new();
        let stream = self.into_stream();

        ReadIntoBytes::new(buffer, stream)
    }

    pub fn read_into_string(self) -> ReadIntoString {
        let future = self.read_into_bytes();

        ReadIntoString::new(future)
    }

    pub async fn read_json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let buffer = self.read_into_bytes().await?;

        serde_json::from_slice(&buffer).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }
}

impl RequestBody {
    pub(crate) fn new(body: EveryBody<Incoming>) -> Self {
        Self { body }
    }
}
