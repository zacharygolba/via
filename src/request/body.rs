use bytes::Bytes;
use http_body::{self, Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyDataStream, BodyExt, BodyStream, Collected, Either, Limited};
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::{bad_request, AnyError, Error};

type RequestBodyKind = Either<Limited<Incoming>, BoxBody<Bytes, AnyError>>;

pub struct RequestBody {
    kind: RequestBodyKind,
}

impl RequestBody {
    pub async fn json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let text = self.to_text().await?;

        serde_json::from_str(&text).map_err(|error| {
            let error = Box::new(error);
            bad_request(error)
        })
    }

    pub async fn to_bytes(self) -> Result<Bytes, Error> {
        Ok(self.collect().await?.to_bytes())
    }

    pub async fn to_text(self) -> Result<String, Error> {
        let utf8 = self.to_vec().await?;

        String::from_utf8(utf8).map_err(|error| {
            let error = Box::new(error);
            bad_request(error)
        })
    }

    pub async fn to_vec(self) -> Result<Vec<u8>, Error> {
        Ok(self.to_bytes().await?.to_vec())
    }

    pub fn data_stream(self) -> BodyDataStream<RequestBody> {
        BodyDataStream::new(self)
    }

    pub fn stream(self) -> BodyStream<RequestBody> {
        BodyStream::new(self)
    }
}

impl RequestBody {
    #[inline]
    pub(crate) fn new(kind: RequestBodyKind) -> Self {
        Self { kind }
    }

    #[inline]
    pub(crate) async fn collect(self) -> Result<Collected<Bytes>, Error> {
        match BodyExt::collect(self).await {
            Ok(collected) => Ok(collected),
            Err(error) => Err(bad_request(error)),
        }
    }
}

impl RequestBody {
    fn project(self: Pin<&mut Self>) -> Pin<&mut RequestBodyKind> {
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
