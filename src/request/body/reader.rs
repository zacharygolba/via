use bytes::{BufMut, Bytes, BytesMut};
use http::HeaderMap;
use http_body::Body;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::Error;

use super::error::error_from_boxed;
use super::RequestBody;

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct BodyReader {
    body: RequestBody,
    payload: Vec<Bytes>,
    trailers: Option<HeaderMap>,
}

#[derive(Debug, Default)]
pub struct ReadToEnd {
    payload: Vec<Bytes>,
    trailers: Option<HeaderMap>,
}

impl BodyReader {
    pub async fn parse_json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        self.await?.parse_json()
    }

    pub async fn into_bytes(self) -> Result<Bytes, Error> {
        Ok(self.await?.into_bytes())
    }

    pub async fn into_text(self) -> Result<String, Error> {
        self.await?.into_text()
    }
}

impl BodyReader {
    pub(crate) fn new(body: RequestBody) -> Self {
        Self {
            body,
            payload: Vec::new(),
            trailers: None,
        }
    }
}

impl Future for BodyReader {
    type Output = Result<ReadToEnd, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut body = Pin::new(&mut this.body);

        loop {
            let frame = match body.as_mut().poll_frame(context) {
                Poll::Ready(Some(Ok(frame))) => frame,
                Poll::Ready(Some(Err(e))) => {
                    let error = error_from_boxed(e);
                    break Poll::Ready(Err(error));
                }
                Poll::Ready(None) => {
                    let payload = this.payload.to_vec();
                    let trailers = this.trailers.take();
                    break Poll::Ready(Ok(ReadToEnd { payload, trailers }));
                }
                Poll::Pending => {
                    break Poll::Pending;
                }
            };

            let trailers = match frame.into_data() {
                Ok(chunk) => {
                    this.payload.push(chunk);
                    continue;
                }
                Err(frame) => match frame.into_trailers() {
                    Err(_) => continue,
                    Ok(map) => map,
                },
            };

            if let Some(existing) = this.trailers.as_mut() {
                existing.extend(trailers);
            } else {
                this.trailers = Some(trailers);
            }
        }
    }
}

impl ReadToEnd {
    pub fn len(&self) -> usize {
        self.payload.iter().map(Bytes::len).sum()
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }

    pub fn parse_json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let payload = self.into_text();

        serde_json::from_str(&payload).map_err(|error| {
            let source = Box::new(error);
            Error::bad_request(source)
        })
    }

    pub fn into_bytes(self) -> Bytes {
        let mut buf = BytesMut::with_capacity(self.len());

        for chunk in self.payload {
            buf.put(chunk);
        }

        buf.freeze()
    }

    pub fn into_text(self) -> Result<String, Error> {
        let mut payload = Vec::with_capacity(self.len());

        for chunk in &self.payload {
            payload.extend_from_slice(chunk);
        }

        String::from_utf8(payload).map_err(|error| {
            let source = Box::new(error);
            Error::bad_request(source)
        })
    }
}
