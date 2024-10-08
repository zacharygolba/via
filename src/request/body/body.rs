use http::StatusCode;
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug, Formatter};

use super::{BodyStream, ReadIntoBytes, ReadIntoString};
use crate::body::AnyBody;
use crate::Error;

pub struct RequestBody {
    body: AnyBody<Incoming>,
}

impl RequestBody {
    pub fn into_inner(self) -> AnyBody<Incoming> {
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

            error.set_status(StatusCode::BAD_REQUEST);
            error
        })
    }
}

impl RequestBody {
    pub(crate) fn new(body: AnyBody<Incoming>) -> Self {
        Self { body }
    }
}

impl Debug for RequestBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.body, f)
    }
}
