use bytes::{Buf, Bytes};
use http::{HeaderMap, StatusCode};
use http_body::{Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyStream, Either, LengthLimitError, Limited};
use hyper::body::Incoming;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use crate::Payload;
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
            let Some(result) = ready!(body.as_mut().poll_frame(context)) else {
                return Poll::Ready(payload.take().ok_or_else(already_read));
            };

            let frame = result.map_err(map_err)?;
            let payload = payload.as_mut().ok_or_else(already_read)?;

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

impl RequestPayload {
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

impl RequestPayload {
    fn new() -> Self {
        Self {
            frames: Default::default(),
            trailers: None,
        }
    }
}

impl Payload for RequestPayload {
    #[inline]
    fn as_slice(&self) -> Option<&[u8]> {
        if let [frame] = &*self.frames {
            Some(frame)
        } else {
            None
        }
    }

    #[inline]
    fn into_vec(mut self) -> Vec<u8> {
        let mut dest = Vec::with_capacity(self.len());

        for frame in &mut self.frames {
            dest.extend_from_slice(frame);
            frame.advance(frame.remaining());
        }

        dest
    }
}
