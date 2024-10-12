use bytes::Bytes;
use http::StatusCode;
use http_body::{self, Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, BodyStream, Either};
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::{AnyError, Error};

pub struct RequestBody {
    kind: Either<Incoming, BoxBody<Bytes, AnyError>>,
}

impl RequestBody {
    pub async fn read_json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let string = self.to_string().await?;

        serde_json::from_str(&string).map_err(|source| {
            let mut error = Error::from(source);

            error.set_status(StatusCode::BAD_REQUEST);
            error
        })
    }

    pub async fn to_bytes(self) -> Result<Bytes, Error> {
        match self.kind.collect().await {
            Ok(buffer) => Ok(buffer.to_bytes()),
            Err(error) => {
                let mut error = Error::from_box_error(error);

                *error.status_mut() = StatusCode::BAD_REQUEST;
                Err(error)
            }
        }
    }

    pub async fn to_string(self) -> Result<String, Error> {
        let data = self.to_vec().await?;

        String::from_utf8(data).map_err(|error| {
            let mut error = Error::from(error);

            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }

    pub async fn to_vec(self) -> Result<Vec<u8>, Error> {
        Ok(self.to_bytes().await?.to_vec())
    }

    pub fn to_stream(self) -> BodyStream<RequestBody> {
        BodyStream::new(self)
    }
}

impl RequestBody {
    #[inline]
    pub(crate) fn new(kind: Either<Incoming, BoxBody<Bytes, AnyError>>) -> Self {
        Self { kind }
    }
}

impl RequestBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut Either<Incoming, BoxBody<Bytes, AnyError>>> {
        let this = self.get_mut();
        let ptr = &mut this.kind;

        Pin::new(ptr)
    }
}

impl Debug for RequestBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.kind, f)
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = AnyError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.kind.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.kind.size_hint()
    }
}
