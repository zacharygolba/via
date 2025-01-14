use bytes::{BufMut, Bytes, BytesMut};
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::response::Builder;
use http::{HeaderName, HeaderValue, StatusCode, Version};
use http_body::Frame;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use super::Response;
use crate::body::{BoxBody, BufferBody, HttpBody, StreamBody};
use crate::error::{BoxError, Error};
use crate::request::RequestBody;

/// Defines how a response body source can be applied to a [ResponseBuilder] to
/// generate a [Response].
///
/// ```
/// use http::header::CONTENT_TYPE;
/// use via::{Error, Next, Request, Response, Pipe};
///
/// async fn echo(request: Request, _: Next) -> Result<Response, Error> {
///     let content_type = request.header(CONTENT_TYPE).cloned();
///     let response = Response::build().headers([(CONTENT_TYPE, content_type)]);
///
///     request.into_body().pipe(response)
/// }
/// ```
///
pub trait Pipe {
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error>;
}

#[derive(Debug, Default)]
pub struct ResponseBuilder {
    builder: Builder,
}

struct JsonPayload<'a, T> {
    data: &'a T,
}

impl Pipe for RequestBody {
    /// Apply `self` to the provided response builder to generate a response.
    ///
    /// The response body will be streamed back to the client with chunked
    /// transfer encoding.
    ///
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Box(BoxBody::new(self)))
    }
}

impl<T> Pipe for T
where
    T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync + 'static,
{
    /// Create a [`StreamBody`] from self and apply it to the provided response
    /// builder to generate a response.
    ///
    /// The response body will be streamed back to the client with chunked
    /// transfer encoding.
    ///
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        response
            .header(TRANSFER_ENCODING, "chunked")
            .body(HttpBody::Box(BoxBody::new(StreamBody::new(self))))
    }
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
            builder: self.builder.header(key, value),
        }
    }

    pub fn headers<I, K, V>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = (K, Option<V>)>,
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        iter.into_iter()
            .fold(self, |builder, (key, option)| match option {
                Some(value) => builder.header(key, value),
                None => builder,
            })
    }

    #[inline]
    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            builder: self.builder.status(status),
        }
    }

    #[inline]
    pub fn version(self, version: Version) -> Self {
        Self {
            builder: self.builder.version(version),
        }
    }

    #[inline]
    pub fn body(self, body: HttpBody<BufferBody>) -> Result<Response, Error> {
        let inner = self.builder.body(body)?;
        Ok(Response::from_inner(inner))
    }

    /// Convert self into a [Response] with an empty payload.
    ///
    #[inline]
    pub fn finish(self) -> Result<Response, Error> {
        self.body(HttpBody::new())
    }

    pub fn json<T: Serialize>(self, data: &T) -> Result<Response, Error> {
        let body = {
            let mut writer = BytesMut::new().writer();
            serde_json::to_writer(&mut writer, &JsonPayload { data })?;

            let buf = writer.into_inner().freeze();
            BufferBody::from_raw(buf)
        };

        self.header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header(CONTENT_LENGTH, body.len())
            .body(HttpBody::Inline(body))
    }

    pub fn html(self, html: String) -> Result<Response, Error> {
        let body = BufferBody::from(html);

        self.header(CONTENT_TYPE, "text/html; charset=utf-8")
            .header(CONTENT_LENGTH, body.len())
            .body(HttpBody::Inline(body))
    }

    pub fn text(self, text: String) -> Result<Response, Error> {
        let body = BufferBody::from(text);

        self.header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(CONTENT_LENGTH, body.len())
            .body(HttpBody::Inline(body))
    }
}

impl<T: Serialize> Serialize for JsonPayload<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("JsonPayload", 1)?;

        state.serialize_field("data", self.data)?;
        state.end()
    }
}
