use bytes::{BufMut, Bytes, BytesMut};
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::response::Builder;
use http::{HeaderName, HeaderValue, StatusCode, Version};
use http_body::{Body, Frame};
use http_body_util::combinators::BoxBody;
use http_body_util::StreamBody;
use serde::Serialize;

use super::Response;
use super::{APPLICATION_JSON, CHUNKED_ENCODING, TEXT_HTML, TEXT_PLAIN};
use crate::body::{BufferBody, HttpBody};
use crate::error::{BoxError, Error};

pub struct ResponseBuilder {
    body: Option<Result<HttpBody<BufferBody>, Error>>,
    inner: Builder,
}

impl ResponseBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            body: None,
            inner: Builder::new(),
        }
    }

    #[inline]
    pub fn body(self, body: HttpBody<BufferBody>) -> Self {
        Self {
            body: Some(Ok(body)),
            ..self
        }
    }

    #[inline]
    pub fn html(self, string: String) -> Self {
        let body = BufferBody::from(string);
        let len = body.len();

        self.body(HttpBody::Inline(body))
            .header(CONTENT_TYPE, TEXT_HTML)
            .header(CONTENT_LENGTH, len)
    }

    #[inline]
    pub fn text(self, string: String) -> Self {
        let body = BufferBody::from(string);
        let len = body.len();

        self.body(HttpBody::Inline(body))
            .header(CONTENT_TYPE, TEXT_PLAIN)
            .header(CONTENT_LENGTH, len)
    }

    #[inline]
    pub fn json<B: Serialize>(self, body: &B) -> Self {
        let (len, json) = {
            let mut writer = BytesMut::new().writer();

            match serde_json::to_writer(&mut writer, body) {
                Ok(_) => {
                    let buf = writer.into_inner().freeze();
                    (buf.len(), BufferBody::from_raw(buf))
                }
                Err(e) => {
                    return Self {
                        body: Some(Err(e.into())),
                        ..self
                    }
                }
            }
        };

        self.body(HttpBody::Inline(json))
            .header(CONTENT_TYPE, APPLICATION_JSON)
            .header(CONTENT_LENGTH, len)
    }

    #[inline]
    pub fn boxed<T>(self, body: T) -> Self
    where
        T: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        self.body(HttpBody::Box(BoxBody::new(body)))
            .header(TRANSFER_ENCODING, CHUNKED_ENCODING)
    }

    /// Build a response from a stream of `Result<Frame<Bytes>, Error>`.
    ///
    #[inline]
    pub fn stream<T>(self, stream: T) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync + 'static,
    {
        let body = BoxBody::new(StreamBody::new(stream));

        self.body(HttpBody::Box(body))
            .header(TRANSFER_ENCODING, CHUNKED_ENCODING)
    }

    #[inline]
    pub fn finish(self) -> Result<Response, Error> {
        let body = self.body.transpose()?.unwrap_or_default();
        Ok(self.inner.body(body)?.into())
    }

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
            ..self
        }
    }

    #[inline]
    pub fn headers<I, K, V>(self, iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        iter.into_iter()
            .fold(self, |builder, (name, value)| builder.header(name, value))
    }

    #[inline]
    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            inner: self.inner.status(status),
            ..self
        }
    }

    #[inline]
    pub fn version(self, version: Version) -> Self {
        Self {
            inner: self.inner.version(version),
            ..self
        }
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
