use bytes::Bytes;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::response::Builder;
use http::{HeaderName, HeaderValue, StatusCode, Version};
use http_body::Body;
use http_body_util::combinators::BoxBody;
use serde::Serialize;

use super::{Response, ResponseBody};
use super::{APPLICATION_JSON, CHUNKED_ENCODING, TEXT_HTML, TEXT_PLAIN};
use crate::error::BoxError;
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
        let result = ResponseBody::try_from(body).map_err(|error| error.into());

        Self {
            body: Some(result),
            inner: self.inner,
        }
    }

    pub fn html(self, string: String) -> Self {
        let mut builder = self.inner;

        builder = builder.header(CONTENT_TYPE, TEXT_HTML);
        builder = builder.header(CONTENT_LENGTH, string.len());

        Self {
            body: Some(Ok(string.into())),
            inner: builder,
        }
    }

    pub fn text(self, string: String) -> Self {
        let mut builder = self.inner;

        builder = builder.header(CONTENT_TYPE, TEXT_PLAIN);
        builder = builder.header(CONTENT_LENGTH, string.len());

        Self {
            body: Some(Ok(string.into())),
            inner: builder,
        }
    }

    pub fn json<B: Serialize>(self, body: &B) -> Self {
        let mut builder = self.inner;
        let json = match serde_json::to_string(body) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Self {
                    body: Some(Err(error.into())),
                    inner: builder,
                }
            }
        };

        builder = builder.header(CONTENT_TYPE, APPLICATION_JSON);
        builder = builder.header(CONTENT_LENGTH, json.len());

        Self {
            body: Some(Ok(json.into())),
            inner: builder,
        }
    }

    /// Build a response from a stream of `Result<Frame<Bytes>, Error>`.
    ///
    pub fn stream<T>(self, body: T) -> Self
    where
        T: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self {
            body: Some(Ok(BoxBody::new(body).into())),
            inner: self.inner.header(TRANSFER_ENCODING, CHUNKED_ENCODING),
        }
    }

    pub fn finish(self) -> Result<Response> {
        let body = self.body.transpose()?.unwrap_or_default();
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

        Self { inner, ..self }
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
