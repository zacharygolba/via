use bytes::Bytes;
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::{HeaderName, HeaderValue, StatusCode, Version};
use http_body::Frame;
use http_body_util::{Either, StreamBody};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use super::response::Response;
use super::BufferBody;
use crate::error::{DynError, Error};
use crate::BoxBody;

/// Define how a type can finalize a [`ResponseBuilder`].
///
/// ```
/// use via::{Next, Request, Response, Pipe};
///
/// async fn echo(request: Request, _: Next) -> via::Result {
///     let mut response = Response::build();
///     request.pipe(response.header("X-Powered-By", "Via"))
/// }
/// ```
///
pub trait Pipe {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error>;
}

#[derive(Debug, Default)]
pub struct ResponseBuilder {
    inner: http::response::Builder,
}

struct JsonPayload<'a, T> {
    data: &'a T,
}

impl ResponseBuilder {
    #[inline]
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            inner: self.inner.header(key, value),
        }
    }

    #[inline]
    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            inner: self.inner.status(status),
        }
    }

    #[inline]
    pub fn version(self, version: Version) -> Self {
        Self {
            inner: self.inner.version(version),
        }
    }

    #[inline]
    pub fn body<T>(self, body: T) -> Result<Response, Error>
    where
        BufferBody: From<T>,
    {
        Ok(self.inner.body(Either::Left(body.into()))?.into())
    }

    #[inline]
    pub fn json(self, data: &impl Serialize) -> Result<Response, Error> {
        let json = serde_json::to_string(&JsonPayload { data })?;

        self.header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header(CONTENT_LENGTH, json.len())
            .body(json)
    }

    #[inline]
    pub fn html(self, data: String) -> Result<Response, Error> {
        self.header(CONTENT_TYPE, "text/html; charset=utf-8")
            .header(CONTENT_LENGTH, data.len())
            .body(data)
    }

    #[inline]
    pub fn text(self, data: String) -> Result<Response, Error> {
        self.header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(CONTENT_LENGTH, data.len())
            .body(data)
    }

    #[inline]
    pub fn boxed(self, body: BoxBody) -> Result<Response, Error> {
        Ok(self.inner.body(Either::Right(body))?.into())
    }

    /// Convert self into a [Response] with an empty payload.
    ///
    #[inline]
    pub fn finish(self) -> Result<Response, Error> {
        self.body(BufferBody::default())
    }
}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Frame<Bytes>, DynError>> + Send + Sync + 'static,
{
    fn pipe(self, builder: ResponseBuilder) -> Result<Response, Error> {
        builder
            .header(TRANSFER_ENCODING, "chunked")
            .boxed(BoxBody::new(StreamBody::new(self)))
    }
}

impl<T: Serialize> Serialize for JsonPayload<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("JsonPayload", 1)?;

        state.serialize_field("data", self.data)?;
        state.end()
    }
}
