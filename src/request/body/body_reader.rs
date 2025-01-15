use bytes::{BufMut, Bytes, BytesMut};
use http::HeaderMap;
use http_body::Body;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::body::HttpBody;
use crate::Error;

use super::json_payload::JsonPayload;
use super::limit_error::error_from_boxed;
use super::RequestBody;

#[derive(Debug, Default)]
pub struct BodyData {
    payload: Vec<Bytes>,
    trailers: Option<HeaderMap>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct BodyReader {
    body: HttpBody<RequestBody>,
    payload: Option<Vec<Bytes>>,
    trailers: Option<HeaderMap>,
}

fn body_already_read() -> Error {
    let message = "request body already read".to_owned();
    Error::internal_server_error(message.into())
}

impl BodyData {
    pub fn len(&self) -> usize {
        self.payload.iter().map(Bytes::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }

    pub fn parse_json<D>(self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let payload = self.into_text()?;

        // Attempt deserialize JSON assuming that type `D` exists in a top-level
        // data field. This is a common pattern so we optimize for it to provide
        // a more convienent API. If you frequently expect `D` to be at the root
        // of the JSON object contained in `payload` and not in a top-level
        // `data` field, we recommend writing a utility function that circumvents
        // the extra call to deserialize. Otherwise, this has no additional
        // overhead.
        serde_json::from_str(&payload)
            // If `D` was contained in a top-level `data` field, unwrap it.
            .map(|json: JsonPayload<D>| json.data)
            // Otherwise, attempt to deserialize `D` from the object at the root
            // of payload. If that also fails, use the original error.
            .or_else(|error| serde_json::from_str(&payload).or(Err(error)))
            // If an error occured, wrap it with `via::Error` and set the status
            // code to 400 Bad Request.
            .map_err(|error| {
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

impl BodyReader {
    pub(crate) fn new(body: HttpBody<RequestBody>) -> Self {
        Self {
            body,
            payload: Some(vec![]),
            trailers: None,
        }
    }
}

impl Future for BodyReader {
    type Output = Result<BodyData, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut body = Pin::new(&mut this.body);

        loop {
            return match body.as_mut().poll_frame(context) {
                Poll::Ready(Some(Ok(frame))) => {
                    let payload = this.payload.as_mut().ok_or_else(body_already_read)?;
                    let frame = match frame.into_data() {
                        Err(frame) => frame,
                        Ok(chunk) => {
                            payload.push(chunk);
                            continue;
                        }
                    };

                    if let Ok(trailers) = frame.into_trailers() {
                        this.trailers.get_or_insert_default().extend(trailers);
                    }

                    continue;
                }

                Poll::Ready(Some(Err(e))) => {
                    let error = error_from_boxed(e);
                    Poll::Ready(Err(error))
                }

                Poll::Ready(None) => Poll::Ready(Ok(BodyData {
                    payload: this.payload.take().ok_or_else(body_already_read)?,
                    trailers: this.trailers.take(),
                })),

                Poll::Pending => Poll::Pending,
            };
        }
    }
}
