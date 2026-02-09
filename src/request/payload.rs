use bytes::{Buf, Bytes};
use http::{HeaderMap, StatusCode};
use http_body::Body;
use http_body_util::{LengthLimitError, Limited};
use hyper::body::Incoming;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, ready};

use crate::error::{BoxError, Error};
use crate::raise;

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Coallesces the bytes in self into a `Vec<u8>`.
    ///
    fn coallesce(self) -> Result<Vec<u8>, Error>;

    /// Coallesces the bytes in self into a `Vec<u8>`. Then, zeroes the
    /// original buffer of each frame in the Payload.
    ///
    fn z_coallesce(self) -> Result<Vec<u8>, Error>;

    /// Deserialize and extract `T` as JSON from the top-level data field of
    /// the object contained by the bytes in self.
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use serde::Deserialize;
    /// # use via::Payload;
    /// #
    /// #[derive(Deserialize)]
    /// struct Cat {
    ///     name: String,
    /// }
    ///
    /// let mut payload = Bytes::copy_from_slice(b"{\"data\":{\"name\":\"Ciro\"}}");
    /// let cat = payload.json::<Cat>().expect("invalid payload");
    ///
    /// println!("Meow, {}!", cat.name);
    /// // => Meow, Ciro!
    /// ```
    ///
    fn json<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.coallesce().and_then(|data| deserialize_json(&data))
    }

    /// Deserialize and extract `T` as JSON from the top-level data field of
    /// the object contained by the bytes in self. Then, zeroes each frame of
    /// the underlying buffer.
    ///
    fn z_json<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.z_coallesce().and_then(|data| deserialize_json(&data))
    }

    /// Copy the bytes in self into an owned, contiguous `String`.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    fn into_utf8(self) -> Result<String, Error> {
        self.coallesce()
            .and_then(|data| match String::from_utf8(data) {
                Ok(string) => Ok(string),
                Err(error) => raise!(400, message = error.to_string()),
            })
    }
}

/// The data and trailers of a request body.
///
pub struct Aggregate {
    payload: RequestPayload,
    _unsend: PhantomData<Rc<()>>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Coalesce {
    body: Limited<Incoming>,
    payload: Option<RequestPayload>,
}

struct RequestPayload {
    frames: SmallVec<[Bytes; 1]>,
    trailers: Option<HeaderMap>,
}

macro_rules! zeroed {
    ($bytes:expr) => {
        match $bytes.try_into_mut() {
            Ok(mut buf) => {
                buf.fill(0);
                buf.advance(buf.len());
            }
            Err(mut buf) => {
                buf.advance(buf.len());
            }
        }
    };
}

fn already_read<T>() -> Result<T, Error> {
    raise!(500, message = "The request body has already been read.")
}

#[inline]
fn deserialize_json<T>(slice: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    #[derive(Deserialize)]
    struct Tagged<D> {
        data: D,
    }

    match serde_json::from_slice(slice) {
        Ok(Tagged { data }) => Ok(data),
        Err(error) => raise!(400, error),
    }
}

fn into_future_error<T>(error: BoxError) -> Result<T, Error> {
    if error.is::<LengthLimitError>() {
        // Payload Too Large
        raise!(413, boxed = error);
    }

    // Bad Request
    raise!(400, boxed = error);
}

impl Aggregate {
    fn new(payload: RequestPayload) -> Self {
        Self {
            payload,
            _unsend: PhantomData,
        }
    }

    pub fn len(&self) -> Option<usize> {
        self.frames()
            .iter()
            .map(Bytes::len)
            .try_fold(0usize, |sum, len| sum.checked_add(len))
    }

    pub fn is_empty(&self) -> bool {
        self.len().is_some_and(|len| len == 0)
    }

    pub fn frames(&self) -> &[Bytes] {
        &self.payload.frames
    }

    pub fn frames_mut(&mut self) -> &mut [Bytes] {
        &mut self.payload.frames
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.payload.trailers.as_ref()
    }

    pub fn mut_trailers(&mut self) -> &mut Option<HeaderMap> {
        &mut self.payload.trailers
    }
}

impl Payload for Aggregate {
    fn coallesce(mut self) -> Result<Vec<u8>, Error> {
        let Some(mut data) = self.len().map(Vec::with_capacity) else {
            raise!(400, message = "payload length would overflow usize::MAX");
        };

        for frame in &mut self.payload.frames {
            data.extend_from_slice(frame);
            frame.advance(frame.len());
        }

        Ok(data)
    }

    fn z_coallesce(self) -> Result<Vec<u8>, Error> {
        let Some(mut data) = self.len().map(Vec::with_capacity) else {
            raise!(400, message = "payload length would overflow usize::MAX");
        };

        for frame in self.payload.frames {
            data.extend_from_slice(&frame);
            zeroed!(frame);
        }

        Ok(data)
    }
}

impl Payload for Bytes {
    fn coallesce(mut self) -> Result<Vec<u8>, Error> {
        let data = self.to_vec();
        self.advance(self.len());
        Ok(data)
    }

    fn z_coallesce(self) -> Result<Vec<u8>, Error> {
        let data = self.to_vec();
        zeroed!(self);
        Ok(data)
    }

    #[inline]
    fn json<T>(mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let result = deserialize_json(&self);
        self.advance(self.len());
        result
    }

    #[inline]
    fn z_json<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let result = deserialize_json(&self);
        zeroed!(self);
        result
    }
}

impl Coalesce {
    pub(super) fn new(body: Limited<Incoming>) -> Self {
        Self {
            body,
            payload: Some(RequestPayload {
                frames: SmallVec::new(),
                trailers: None,
            }),
        }
    }
}

impl Future for Coalesce {
    type Output = Result<Aggregate, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let Self { body, payload } = self.get_mut();
        let mut body = Pin::new(body);

        loop {
            let Some(result) = ready!(body.as_mut().poll_frame(context)) else {
                return Poll::Ready(match payload.take() {
                    Some(payload) => Ok(Aggregate::new(payload)),
                    None => already_read(),
                });
            };

            let frame = result.or_else(into_future_error)?;
            let payload = payload.as_mut().map_or_else(already_read, Ok)?;
            let trailers = match frame.into_data() {
                Ok(data) => {
                    payload.frames.push(data);
                    continue;
                }
                Err(frame) => {
                    let Ok(trailers) = frame.into_trailers() else {
                        return Poll::Ready(Err(Error::new(
                            StatusCode::BAD_REQUEST,
                            "unexpected frame type received while reading the request body",
                        )));
                    };

                    trailers
                }
            };

            if let Some(existing) = payload.trailers.as_mut() {
                existing.extend(trailers);
            } else {
                payload.trailers = Some(trailers);
            }
        }
    }
}
