use bytes::Bytes;
use futures_core::ready;
use http::HeaderMap;
use http_body::Body;
use serde::de::DeserializeOwned;
use serde::de::{self, Deserialize, Deserializer, MapAccess, Visitor};
use std::fmt::{self, Formatter};
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::{util, BoxBody};
use crate::error::Error;

/// The entire contents of a request body, in-memory.
///
#[derive(Debug)]
pub struct BodyData {
    payload: Vec<Bytes>,
    trailers: Option<HeaderMap>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct BodyReader {
    body: BoxBody,
    data: Option<BodyData>,
}

struct JsonPayload<T> {
    data: T,
}

struct JsonPayloadVisitor<T> {
    marker: PhantomData<T>,
}

fn already_read() -> Error {
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

    pub fn parse_json<D>(&self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let payload = self.to_utf8()?;

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
            .or_else(|e| serde_json::from_str(&payload).or(Err(e)))
            // If an error occured, wrap it with `via::Error` and set the status
            // code to 400 Bad Request.
            .map_err(|e| Error::bad_request(Box::new(e)))
    }

    pub fn to_utf8(&self) -> Result<String, Error> {
        match String::from_utf8(self.to_vec()) {
            Ok(utf8) => Ok(utf8),
            Err(e) => Err(Error::bad_request(Box::new(e))),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(self.len());

        for chunk in &self.payload {
            vec.extend_from_slice(chunk);
        }

        vec
    }
}

impl BodyReader {
    pub(crate) fn new(body: BoxBody) -> Self {
        Self {
            body,
            data: Some(BodyData {
                payload: Vec::new(),
                trailers: None,
            }),
        }
    }
}

impl BodyReader {
    fn project(self: Pin<&mut Self>) -> (Pin<&mut BoxBody>, &mut Option<BodyData>) {
        let this = self.get_mut();
        (Pin::new(&mut this.body), &mut this.data)
    }
}

impl Future for BodyReader {
    type Output = Result<BodyData, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let (mut body, data) = self.project();

        loop {
            return match ready!(body.as_mut().poll_frame(context)) {
                None => Poll::Ready(data.take().ok_or_else(already_read)),
                Some(Err(e)) => Poll::Ready(Err(util::map_err(e))),
                Some(Ok(frame)) => {
                    let output = data.as_mut().ok_or_else(already_read)?;

                    match frame.into_data() {
                        Ok(data) => {
                            output.payload.push(data);
                        }
                        Err(frame) => {
                            let trailers = frame.into_trailers().unwrap();
                            if let Some(existing) = output.trailers.as_mut() {
                                existing.extend(trailers);
                            } else {
                                output.trailers = Some(trailers);
                            }
                        }
                    }

                    continue;
                }
            };
        }
    }
}

impl<'de, T> Deserialize<'de> for JsonPayload<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "JsonPayload",
            &["data"],
            JsonPayloadVisitor {
                marker: PhantomData,
            },
        )
    }
}

impl<'de, T> Visitor<'de> for JsonPayloadVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = JsonPayload<T>;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a JSON object with a `data` field")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut value = None;

        while let Some(key) = map.next_key::<String>()? {
            if key == "data" {
                // Similar to the JSON API in web browsers. Duplicate fields
                // result in subsequent keys overriding the value previously
                // defined in the payload.
                value = Some(map.next_value()?);
            } else {
                // Skip unknown fields.
                map.next_value::<de::IgnoredAny>()?;
            }
        }

        match value {
            Some(data) => Ok(JsonPayload { data }),
            None => Err(de::Error::missing_field("data")),
        }
    }
}
