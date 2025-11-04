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

#[derive(Debug)]
pub struct RequestBody(Either<Limited<Incoming>, BoxBody<Bytes, BoxError>>);

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct IntoFuture {
    body: RequestBody,
    payload: Option<DataAndTrailers>,
}

/// The data and trailers of a request body.
///
#[derive(Debug)]
pub struct DataAndTrailers {
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
    fn new(body: RequestBody, payload: DataAndTrailers) -> Self {
        Self {
            body,
            payload: Some(payload),
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

    pub fn into_future(self) -> IntoFuture {
        IntoFuture::new(self, DataAndTrailers::new())
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

impl DataAndTrailers {
    fn new() -> Self {
        Self {
            frames: Default::default(),
            trailers: None,
        }
    }
}

impl Payload for DataAndTrailers {
    fn copy_to_bytes(mut self) -> Bytes {
        let mut dest = BytesMut::with_capacity(self.len());

        for frame in &mut self.frames {
            let remaining = frame.len();
            let detached = frame.split_to(remaining);

            dest.extend_from_slice(detached.as_ref());
        }

        dest.freeze()
    }

    fn into_vec(mut self) -> Vec<u8> {
        let mut dest = Vec::with_capacity(self.len());

        for frame in &mut self.frames {
            let remaining = frame.len();
            let detached = frame.split_to(remaining);

            dest.extend_from_slice(detached.as_ref());
        }

        dest
    }
}
