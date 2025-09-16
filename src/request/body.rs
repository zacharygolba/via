use bytes::Bytes;
use http::{HeaderMap, StatusCode};
use http_body::{Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyStream, Either, LengthLimitError, Limited};
use hyper::body::Incoming;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::{BoxError, Error};

#[derive(Debug)]
pub struct RequestBody(Either<Limited<Incoming>, BoxBody<Bytes, BoxError>>);

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct IntoFuture {
    body: RequestBody,
    payload: Option<RequestPayload>,
}

/// The entire contents of a request body, in-memory.
///
#[derive(Debug)]
pub struct RequestPayload {
    frames: Vec<Bytes>,
    trailers: Option<HeaderMap>,
}

#[derive(Deserialize)]
struct JsonPayload<T> {
    data: T,
}

fn already_read() -> Error {
    crate::error!(500, "The request body has already been read.")
}

fn map_err(error: BoxError) -> Error {
    if error.is::<LengthLimitError>() {
        Error::new(StatusCode::PAYLOAD_TOO_LARGE, error)
    } else {
        Error::new(StatusCode::BAD_REQUEST, error)
    }
}

impl RequestPayload {
    pub fn parse_json<D>(&self) -> Result<D, Error>
    where
        D: DeserializeOwned,
    {
        let json = match self.as_str()? {
            Some(utf8) => Cow::Borrowed(utf8),
            None => Cow::Owned(self.to_string()?),
        };

        // Attempt deserialize JSON assuming that type `D` exists in a top
        // level data field. This is a common pattern so we optimize for it to
        // provide a more convenient API. If you frequently expect `D` to be at
        // the root of the JSON object contained in `payload` and not in a top-
        // level `data` field, we recommend writing a utility function that
        // circumvents the extra call to deserialize. Otherwise, this has no
        // additional overhead.
        serde_json::from_str(&json)
            // If `D` was contained in a top-level `data` field, unwrap it.
            .map(|object: JsonPayload<D>| object.data)
            // Otherwise, attempt to deserialize `D` from the object at the
            // root of payload. If that also fails, use the original error.
            .or_else(|error| serde_json::from_str(&json).or(Err(error)))
            // If an error occurred, wrap it with `via::Error` and set the status
            // code to 400 Bad Request.
            .map_err(Error::bad_request)
    }

    pub fn to_string(&self) -> Result<String, Error> {
        String::from_utf8(self.to_vec()).map_err(Error::bad_request)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.frames.iter().map(Bytes::len).sum());

        for frame in &self.frames {
            buf.extend_from_slice(frame);
        }

        buf
    }

    pub fn trailers(&self) -> Option<&HeaderMap> {
        self.trailers.as_ref()
    }

    /// Return the entire body as a slice if it is composed of a single frame.
    ///
    pub fn as_slice(&self) -> Option<&[u8]> {
        if let [slice] = self.frames.as_slice() {
            Some(slice)
        } else {
            None
        }
    }

    /// Return the entire body as a str if it is composed of a single frame and
    /// is valid UTF-8.
    ///
    pub fn as_str(&self) -> Result<Option<&str>, Error> {
        self.as_slice()
            .map(str::from_utf8)
            .transpose()
            .map_err(Error::bad_request)
    }
}

impl RequestPayload {
    fn new() -> Self {
        Self {
            frames: Vec::new(),
            trailers: None,
        }
    }
}

impl IntoFuture {
    fn new(body: RequestBody, payload: RequestPayload) -> Self {
        Self {
            body,
            payload: Some(payload),
        }
    }
}

impl Future for IntoFuture {
    type Output = Result<RequestPayload, Error>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let Self { body, payload } = self.get_mut();
        let mut body = Pin::new(body);

        loop {
            break match body.as_mut().poll_frame(context).map_err(map_err)? {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(payload.take().ok_or_else(already_read)),
                Poll::Ready(Some(frame)) => {
                    let payload = payload.as_mut().ok_or_else(already_read)?;

                    match frame.into_data() {
                        Ok(data) => payload.frames.push(data),
                        Err(frame) => {
                            let trailers = frame.into_trailers().unwrap();
                            if let Some(existing) = payload.trailers.as_mut() {
                                existing.extend(trailers);
                            } else {
                                payload.trailers = Some(trailers);
                            }
                        }
                    }

                    continue;
                }
            };
        }
    }
}

impl RequestBody {
    /// Consume the request body and return a dynamically-dispatched
    /// [`BoxBody`].
    ///
    pub fn boxed(self) -> BoxBody<Bytes, BoxError> {
        match self {
            Self(Either::Left(inline)) => BoxBody::new(inline),
            Self(Either::Right(boxed)) => boxed,
        }
    }

    pub fn into_stream(self) -> BodyStream<Self> {
        BodyStream::new(self)
    }

    pub async fn into_future(self) -> Result<RequestPayload, Error> {
        IntoFuture::new(self, RequestPayload::new()).await
    }
}

impl RequestBody {
    #[inline]
    pub(crate) fn new(body: Limited<Incoming>) -> Self {
        Self(Either::Left(body))
    }

    #[inline]
    pub(crate) fn map<U, F>(self, map: F) -> Self
    where
        F: FnOnce(RequestBody) -> U,
        U: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self(Either::Right(BoxBody::new(map(self))))
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let Self(body) = self.get_mut();
        Pin::new(body).poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }
}
