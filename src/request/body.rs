use bytes::Bytes;
use http_body::{self, Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyDataStream, BodyExt, BodyStream, Collected};
use hyper::body::Incoming;
use serde::de::DeserializeOwned;
use std::fmt::{self, Display, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::HttpBody;
use crate::error::{BoxError, Error};

#[derive(Clone, Copy, Debug)]
struct PayloadTooLargeError;

#[derive(Debug)]
pub struct HyperBody {
    body: Incoming,
}

#[derive(Debug)]
pub struct RequestBody {
    remaining: usize,
    body: HttpBody<HyperBody>,
}

impl HyperBody {
    #[inline]
    pub(crate) fn new(body: Incoming) -> Self {
        Self { body }
    }
}

impl Body for HyperBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        Pin::new(&mut this.body)
            .poll_frame(cx)
            .map_err(|e| e.into())
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl RequestBody {
    pub async fn json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let text = self.to_text().await?;

        match serde_json::from_str(&text) {
            Ok(json) => Ok(json),
            Err(error) => {
                let source = Box::new(error);
                Err(Error::bad_request(source))
            }
        }
    }

    pub async fn to_bytes(self) -> Result<Bytes, Error> {
        Ok(self.collect().await?.to_bytes())
    }

    pub async fn to_text(self) -> Result<String, Error> {
        let utf8 = self.to_vec().await?;

        match String::from_utf8(utf8) {
            Ok(text) => Ok(text),
            Err(error) => {
                let source = Box::new(error);
                Err(Error::bad_request(source))
            }
        }
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
    pub(crate) fn new(remaining: usize, body: HttpBody<HyperBody>) -> Self {
        Self { remaining, body }
    }

    #[inline]
    pub(crate) fn map<F>(self, map: F) -> Self
    where
        F: FnOnce(HttpBody<HyperBody>) -> BoxBody<Bytes, BoxError>,
    {
        Self {
            body: HttpBody::Box(map(self.body)),
            ..self
        }
    }

    #[inline]
    pub(crate) async fn collect(self) -> Result<Collected<Bytes>, Error> {
        let source = match BodyExt::collect(self).await {
            Ok(collected) => return Ok(collected),
            Err(error) => error,
        };

        if let Some(&PayloadTooLargeError) = source.downcast_ref() {
            Err(Error::payload_too_large(source))
        } else {
            Err(Error::bad_request(source))
        }
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();

        match Pin::new(&mut this.body).poll_frame(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(frame))) => {
                if let Some(data) = frame.data_ref() {
                    let frame_len = data.len();

                    if this.remaining < frame_len {
                        let error = Box::new(PayloadTooLargeError);
                        return Poll::Ready(Some(Err(error)));
                    }

                    this.remaining -= frame_len;
                }

                Poll::Ready(Some(Ok(frame)))
            }
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
        }
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.body.size_hint()
    }
}

impl std::error::Error for PayloadTooLargeError {}

impl Display for PayloadTooLargeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Payload Too Large")
    }
}
