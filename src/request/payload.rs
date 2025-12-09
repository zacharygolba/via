use bytes::{Buf, Bytes, BytesMut};
use http::HeaderMap;
use http_body::Body;
use http_body_util::{LengthLimitError, Limited};
use hyper::body::Incoming;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::future::Future;
use std::pin::Pin;
use std::slice;
use std::task::{Context, Poll, ready};

use crate::error::{BoxError, Error};
use crate::raise;

/// Interact with data received from a client.
///
pub trait Payload {
    /// Copy the bytes in self into a unique, contiguous `Bytes` instance.
    ///
    fn copy_to_unique(&mut self) -> Result<Bytes, Error>;

    /// Copy the bytes in self into an owned, contiguous `String`.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    fn copy_to_utf8(&mut self) -> Result<String, Error> {
        let vec = self.copy_to_vec()?;
        String::from_utf8(vec).map_err(|error| {
            let error = error.utf8_error();
            Error::from_utf8_error(error)
        })
    }

    /// Copy the bytes in self into a contiguous `Vec<u8>`.
    ///
    fn copy_to_vec(&mut self) -> Result<Vec<u8>, Error> {
        self.copy_to_unique().map(Vec::from)
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
    fn json<T>(&mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        deserialize_json(&self.copy_to_unique()?)
    }
}

/// The data and trailers of a request body.
///
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

impl DataAndTrailers {
    pub fn len(&self) -> Option<usize> {
        self.iter()
            .try_fold(0usize, |len, frame| len.checked_add(frame.len()))
    }

    pub fn is_empty(&self) -> bool {
        self.len().is_some_and(|len| len == 0)
    }

    pub fn iter(&self) -> slice::Iter<'_, Bytes> {
        self.frames.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, Bytes> {
        self.frames.iter_mut()
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }
}

impl Payload for DataAndTrailers {
    fn copy_to_unique(&mut self) -> Result<Bytes, Error> {
        let Some(mut bytes) = self.len().map(BytesMut::with_capacity) else {
            raise!(400, message = "payload length would overflow usize::MAX.");
        };

        for frame in self.iter_mut() {
            bytes.extend_from_slice(&*frame);
            frame.advance(frame.len());
        }

        Ok(bytes.freeze())
    }

    fn json<T>(&mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        match &mut *self.frames {
            [frame] => frame.json(),
            _ => deserialize_json(&self.copy_to_unique()?),
        }
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
            let trailers = match frame.into_data() {
                Ok(data) => {
                    payload.frames.push(data);
                    continue;
                }
                Err(frame) => match frame.into_trailers() {
                    Ok(trailers) => trailers,
                    Err(_) => unreachable!(),
                },
            };

            if let Some(existing) = payload.trailers.as_mut() {
                existing.extend(trailers);
            } else {
                payload.trailers = Some(trailers);
            }
        }
    }
}

impl Payload for Bytes {
    fn copy_to_unique(&mut self) -> Result<Bytes, Error> {
        let bytes = Bytes::copy_from_slice(&*self);

        self.advance(self.len());

        Ok(bytes)
    }

    fn copy_to_vec(&mut self) -> Result<Vec<u8>, Error> {
        let vec = self.to_vec();

        self.advance(self.len());

        Ok(vec)
    }

    fn json<T>(&mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let remaining = self.len();
        let result = deserialize_json(&*self);

        self.advance(remaining);

        result
    }
}
