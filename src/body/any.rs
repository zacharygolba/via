use bytes::Bytes;
use http::StatusCode;
use hyper::body::{Body, Frame, Incoming, SizeHint};
use serde::de::DeserializeOwned;
use std::pin::Pin;
use std::task::Poll;

use super::aggregate::{ReadIntoBytes, ReadIntoString};
use super::stream::{BodyDataStream, BodyStream};
use super::{Boxed, Buffered};
use crate::Error;

/// A sum type that can represent any [Request](crate::Request) or
/// [Response](crate::Response) body.
#[non_exhaustive]
#[must_use = "streams do nothing unless polled"]
pub enum AnyBody<B> {
    Boxed(Boxed),
    Inline(B),
}

enum AnyBodyProjection<'a, B> {
    Boxed(Pin<&'a mut Boxed>),
    Inline(Pin<&'a mut B>),
}

impl AnyBody<Buffered> {
    pub fn new() -> Self {
        Self::Inline(Default::default())
    }
}

impl AnyBody<Box<Incoming>> {
    pub fn into_stream(self) -> BodyStream {
        BodyStream::new(self)
    }

    pub fn into_data_stream(self) -> BodyDataStream {
        let stream = self.into_stream();
        BodyDataStream::new(stream)
    }

    pub fn read_into_bytes(self) -> ReadIntoBytes {
        let buffer = Vec::new();
        let stream = self.into_data_stream();

        ReadIntoBytes::new(buffer, stream)
    }

    pub fn read_into_string(self) -> ReadIntoString {
        let future = self.read_into_bytes();

        ReadIntoString::new(future)
    }

    pub async fn read_json<B>(self) -> Result<B, Error>
    where
        B: DeserializeOwned,
    {
        let buffer = self.read_into_bytes().await?;

        serde_json::from_slice(&buffer).map_err(|source| {
            let mut error = Error::from(source);
            *error.status_mut() = StatusCode::BAD_REQUEST;
            error
        })
    }
}

impl<B> AnyBody<B> {
    fn project(self: Pin<&mut Self>) -> AnyBodyProjection<B> {
        let this = unsafe {
            //
            // Safety:
            //
            // We need a mutable reference to self to project the inner body type
            // for the variant that is currently stored in the enum. The pointer
            // is never moved or deallocated for any of the variants.
            //
            self.get_unchecked_mut()
        };

        match this {
            Self::Boxed(ptr) => {
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The implementation of `Body::poll_frame` for `Boxed`
                    // operates directly on a `Pin<Box<dyn Body>>` and does not
                    // move the pointer or the boxed value out of the pinned ref.
                    //
                    Pin::new_unchecked(ptr)
                };

                AnyBodyProjection::Boxed(pin)
            }
            Self::Inline(ptr) => {
                let pin = unsafe {
                    //
                    // Safety:
                    //
                    // The implementation of `Body::poll_frame` for `Buffered`
                    // treats the buffer as pinned and does not move data out of
                    // the buffer or the pinned mutable reference. Instead, data
                    // is copied out of the buffer and the cursor is advanced to
                    // the next frame.
                    //
                    Pin::new_unchecked(ptr)
                };

                AnyBodyProjection::Inline(pin)
            }
        }
    }
}

impl<B, E> Body for AnyBody<B>
where
    B: Body<Data = Bytes, Error = E>,
    E: Into<Error>,
{
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.project() {
            AnyBodyProjection::Boxed(boxed) => boxed.poll_frame(context),
            AnyBodyProjection::Inline(body) => {
                body.poll_frame(context).map_err(|error| error.into())
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Boxed(boxed) => boxed.is_end_stream(),
            Self::Inline(body) => body.is_end_stream(),
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Boxed(boxed) => boxed.size_hint(),
            Self::Inline(body) => body.size_hint(),
        }
    }
}

impl<B> From<Boxed> for AnyBody<B> {
    fn from(boxed: Boxed) -> Self {
        Self::Boxed(boxed)
    }
}

impl Default for AnyBody<Buffered> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<T> for AnyBody<Buffered>
where
    Buffered: From<T>,
{
    fn from(body: T) -> Self {
        Self::Inline(body.into())
    }
}
