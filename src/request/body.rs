use bytes::{Bytes, BytesMut};
use http::HeaderMap;
use http_body::Body;
use http_body_util::{LengthLimitError, Limited};
use hyper::body::Incoming;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use crate::error::{BoxError, Error};
use crate::raise;

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Copy the bytes in self into a unique, contiguous `Bytes` instance.
    ///
    fn into_bytes(self) -> Bytes;

    /// Copy the bytes in self into an owned, contiguous `String`.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    fn into_utf8(self) -> Result<String, Error> {
        String::from_utf8(self.into_vec()).or_else(|error| raise!(400, error))
    }

    /// Copy the bytes in self into a contiguous `Vec<u8>`.
    ///
    fn into_vec(self) -> Vec<u8> {
        self.into_bytes().into()
    }

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
        deserialize_json(&self.into_vec())
    }
}

/// The data and trailers of a request body.
///
#[derive(Debug)]
pub struct DataAndTrailers {
    frames: SmallVec<[Bytes; 1]>,
    trailers: Option<HeaderMap>,
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct IntoFuture {
    body: Limited<Incoming>,
    payload: Option<DataAndTrailers>,
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
    struct Json<D> {
        data: D,
    }

    match serde_json::from_slice(slice) {
        Ok(Json { data }) => Ok(data),
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

impl DataAndTrailers {
    pub fn len(&self) -> usize {
        self.frames.iter().map(Bytes::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }
}

impl Payload for DataAndTrailers {
    fn into_bytes(self) -> Bytes {
        let mut buf = BytesMut::new();

        for frame in self.frames {
            buf.extend_from_slice(&frame);
        }

        buf.freeze()
    }

    fn into_vec(self) -> Vec<u8> {
        let mut vec = Vec::new();

        for frame in self.frames {
            vec.extend_from_slice(&frame);
        }

        vec
    }
}

impl IntoFuture {
    pub(super) fn new(body: Limited<Incoming>) -> Self {
        Self {
            body,
            payload: Some(DataAndTrailers {
                frames: SmallVec::new(),
                trailers: None,
            }),
        }
    }
}

impl Future for IntoFuture {
    type Output = Result<DataAndTrailers, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let Self { body, payload } = self.get_mut();
        let mut body = Pin::new(body);

        loop {
            let Some(result) = ready!(body.as_mut().poll_frame(context)) else {
                return Poll::Ready(payload.take().map_or_else(already_read, Ok));
            };

            let frame = result.or_else(into_future_error)?;
            let payload = payload.as_mut().map_or_else(already_read, Ok)?;

            match frame.into_data() {
                Ok(data) => payload.frames.push(data),
                Err(frame) => {
                    // If the frame isn't a data frame, it must be trailers.
                    let Ok(trailers) = frame.into_trailers() else {
                        unreachable!()
                    };

                    if let Some(existing) = payload.trailers.as_mut() {
                        existing.extend(trailers);
                    } else {
                        payload.trailers = Some(trailers);
                    }
                }
            };
        }
    }
}

impl Payload for Bytes {
    fn into_bytes(mut self) -> Bytes {
        let remaining = self.len();
        let mut dest = BytesMut::with_capacity(remaining);

        dest.extend_from_slice(&self.split_to(remaining));
        dest.freeze()
    }

    fn into_vec(mut self) -> Vec<u8> {
        let remaining = self.len();
        let mut dest = Vec::with_capacity(remaining);

        dest.extend_from_slice(&self.split_to(remaining));
        dest
    }

    fn json<T>(mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        deserialize_json(&self.split_to(self.len()))
    }
}
