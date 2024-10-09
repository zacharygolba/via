use bytes::Bytes;
use core::str;
use http::StatusCode;
use http_body::{Body, Frame};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, BodyStream, Either};
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

type EitherRequestBody = Either<Incoming, UnsyncBoxBody<Bytes, Error>>;

pub struct RequestBody {
    body: Either<Incoming, UnsyncBoxBody<Bytes, Error>>,
}

impl RequestBody {
    pub fn into_inner(self) -> EitherRequestBody {
        self.body
    }

    pub fn into_stream(self) -> BodyStream<EitherRequestBody> {
        BodyStream::new(self.body)
    }

    pub async fn into_bytes(self) -> Result<Bytes, Error> {
        match self.body.collect().await {
            Ok(buffer) => Ok(buffer.to_bytes()),
            Err(error) => {
                let mut error = Error::from_box_error(error);

                *error.status_mut() = StatusCode::BAD_REQUEST;
                Err(error)
            }
        }
    }

    pub async fn into_string(self) -> Result<String, Error> {
        let bytes = self.into_bytes().await?;

        match str::from_utf8(&bytes) {
            Ok(str) => Ok(str.to_owned()),
            Err(error) => {
                let mut error = Error::from(error);

                *error.status_mut() = StatusCode::BAD_REQUEST;
                Err(error)
            }
        }
    }

    pub async fn into_vec(self) -> Result<Vec<u8>, Error> {
        let bytes = self.into_bytes().await?;
        Ok(Vec::from(bytes))
    }

    pub async fn read_json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let bytes = self.into_bytes().await?;

        serde_json::from_slice(&bytes).map_err(|source| {
            let mut error = Error::from(source);

            error.set_status(StatusCode::BAD_REQUEST);
            error
        })
    }
}

impl RequestBody {
    pub(crate) fn new(body: Incoming) -> Self {
        Self {
            body: Either::Left(body),
        }
    }

    pub(crate) fn from_dyn<B>(body: B) -> Self
    where
        B: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        Self {
            body: Either::Right(UnsyncBoxBody::new(body)),
        }
    }
}

impl RequestBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut EitherRequestBody> {
        let this = self.get_mut();
        let ptr = &mut this.body;

        Pin::new(ptr)
    }
}

impl Debug for RequestBody {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.body, f)
    }
}

impl Body for RequestBody {
    type Data = <EitherRequestBody as Body>::Data;
    type Error = <EitherRequestBody as Body>::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        self.project().poll_frame(context)
    }
}
