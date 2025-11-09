use bytes::{Bytes, BytesMut};
use http::HeaderMap;
use http_body::Body;
use http_body_util::{LengthLimitError, Limited};
use hyper::body::Incoming;
use smallvec::SmallVec;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use crate::error::{BoxError, Error};
use crate::{Payload, raise};

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct IntoFuture {
    body: Limited<Incoming>,
    payload: Option<RequestPayload>,
}

/// The data and trailers of a request body.
///
#[derive(Debug)]
pub struct RequestPayload {
    frames: SmallVec<[Bytes; 1]>,
    trailers: Option<HeaderMap>,
}

fn already_read<T>() -> Result<T, Error> {
    raise!(500, message = "The request body has already been read.")
}

fn into_future_error<T>(error: BoxError) -> Result<T, Error> {
    if error.is::<LengthLimitError>() {
        // Payload Too Large
        raise!(413, boxed = error);
    }

    // Bad Request
    raise!(400, boxed = error);
}

impl IntoFuture {
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

impl Future for IntoFuture {
    type Output = Result<RequestPayload, Error>;

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

impl Payload for RequestPayload {
    fn copy_to_bytes(self) -> Bytes {
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
