use bytes::Bytes;
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::response::Builder;
use http::{HeaderName, HeaderValue, StatusCode, Version};
use http_body::Frame;
use serde::Serialize;

use super::stream_adapter::StreamAdapter;
use super::{Response, ResponseBody};
use super::{APPLICATION_JSON, CHUNKED_ENCODING, TEXT_HTML, TEXT_PLAIN};
use crate::body::Boxed;
use crate::{Error, Result};

pub struct ResponseBuilder {
    body: Option<Result<ResponseBody>>,
    inner: Builder,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Self {
            body: None,
            inner: Builder::new(),
        }
    }

    pub fn body<T>(self, body: T) -> Self
    where
        ResponseBody: TryFrom<T>,
        <ResponseBody as TryFrom<T>>::Error: Into<Error>,
    {
        Self {
            body: Some(ResponseBody::try_from(body).map_err(Into::into)),
            inner: self.inner,
        }
    }

    pub fn html(self, body: String) -> Self {
        let mut builder = self.inner;
        let buf = body.into_bytes();

        builder = builder.header(CONTENT_TYPE, TEXT_HTML);
        builder = builder.header(CONTENT_LENGTH, buf.len());

        Self {
            body: Some(Ok(ResponseBody::from_vec(buf))),
            inner: builder,
        }
    }

    pub fn text(self, body: String) -> Self {
        let mut builder = self.inner;
        let buf = body.into_bytes();

        builder = builder.header(CONTENT_TYPE, TEXT_PLAIN);
        builder = builder.header(CONTENT_LENGTH, buf.len());

        Self {
            body: Some(Ok(ResponseBody::from_vec(buf))),
            inner: builder,
        }
    }

    pub fn json<B: Serialize>(self, body: &B) -> Self {
        let mut builder = self.inner;
        let buf = match serde_json::to_vec(body) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Self {
                    body: Some(Err(error.into())),
                    inner: builder,
                }
            }
        };

        builder = builder.header(CONTENT_TYPE, APPLICATION_JSON);
        builder = builder.header(CONTENT_LENGTH, buf.len());

        Self {
            body: Some(Ok(ResponseBody::from_vec(buf))),
            inner: builder,
        }
    }

    /// Build a response from a stream of `Result<Frame<Bytes>, Error>`.
    ///
    /// If you want to use a stream that is `!Unpin`, you can use the [`Boxed`]
    /// body in combination with a [`ResponseBuilder`]. However, you must ensure
    ///
    pub fn stream<S, E>(self, stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let mut builder = self.inner;
        let stream_body = Boxed::new(StreamAdapter::new(stream));

        builder = builder.header(TRANSFER_ENCODING, CHUNKED_ENCODING);

        Self {
            body: Some(Ok(ResponseBody::from_boxed(stream_body))),
            inner: builder,
        }
    }

    pub fn finish(self) -> Result<Response, Error> {
        let body = match self.body {
            Some(body) => body?,
            None => ResponseBody::new(),
        };

        Ok(self.inner.body(body)?.into())
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            body: self.body,
            inner: self.inner.header(key, value),
        }
    }

    pub fn headers<I, K, V>(self, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        let inner = headers
            .into_iter()
            .fold(self.inner, |builder, (key, value)| {
                builder.header(key, value)
            });

        Self {
            inner,
            body: self.body,
        }
    }

    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            body: self.body,
            inner: self.inner.status(status),
        }
    }

    pub fn version(self, version: Version) -> Self {
        Self {
            body: self.body,
            inner: self.inner.version(version),
        }
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}
