use http::{
    header::{HeaderName, HeaderValue},
    response::Builder,
    StatusCode, Version,
};

use super::{body::Body, Response};
use crate::{Error, Result};

pub struct ResponseBuilder {
    body: Option<Result<Body>>,
    inner: Builder,
}

impl ResponseBuilder {
    pub fn body<T>(self, body: T) -> Self
    where
        Body: TryFrom<T>,
        <Body as TryFrom<T>>::Error: Into<Error>,
    {
        Self {
            body: Some(Body::try_from(body).map_err(Into::into)),
            inner: self.inner,
        }
    }

    pub fn finish(mut self) -> Result<Response> {
        let body = self.body.take().unwrap_or_else(|| Ok(Body::new()))?;

        Ok(Response {
            inner: self.inner.body(body)?,
        })
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

impl ResponseBuilder {
    pub(crate) fn new() -> Self {
        Self {
            body: None,
            inner: Builder::new(),
        }
    }

    pub(crate) fn with_body(body: Result<Body>) -> Self {
        Self {
            body: Some(body),
            inner: Builder::new(),
        }
    }
}
