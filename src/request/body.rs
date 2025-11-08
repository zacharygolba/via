use bytes::{Bytes, BytesMut};
use http::HeaderMap;
use http_body::{Body, Frame, SizeHint};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyStream, Either, LengthLimitError, Limited};
use hyper::body::Incoming;
use smallvec::SmallVec;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use crate::error::{BoxError, Error};
use crate::{Payload, raise};

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct IntoFuture {
    body: RequestBody,
    payload: Option<RequestPayload>,
}

#[derive(Debug)]
pub struct RequestBody {
    pub(super) kind: Either<Limited<Incoming>, BoxBody<Bytes, BoxError>>,
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

impl RequestBody {
    #[inline]
    pub(crate) fn new(body: Limited<Incoming>) -> Self {
        Self {
            kind: Either::Left(body),
        }
    }

    #[inline]
    pub fn into_stream(self) -> BodyStream<Self> {
        BodyStream::new(self)
    }

    #[inline]
    pub fn into_future(self) -> IntoFuture {
        IntoFuture {
            body: self,
            payload: Some(RequestPayload {
                frames: Default::default(),
                trailers: None,
            }),
        }
    }
}

impl Body for RequestBody {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        context: &mut Context,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.kind).poll_frame(context)
    }

    fn is_end_stream(&self) -> bool {
        self.kind.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.kind.size_hint()
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
        self.frames
            .into_iter()
            .fold(BytesMut::new(), |mut buf, mut frame| {
                buf.extend_from_slice(&frame.split_to(frame.len()));
                buf
            })
            .freeze()
    }

    fn into_vec(self) -> Vec<u8> {
        self.frames
            .into_iter()
            .fold(Vec::new(), |mut buf, mut frame| {
                buf.extend_from_slice(&frame.split_to(frame.len()));
                buf
            })
    }
}
