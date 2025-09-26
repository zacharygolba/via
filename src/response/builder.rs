use cookie::CookieJar;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, StatusCode, Version};
use serde::Serialize;

use super::body::ResponseBody;
use super::response::Response;
use crate::error::Error;

#[derive(Debug, Default)]
pub struct ResponseBuilder {
    inner: http::response::Builder,
}

#[derive(Serialize)]
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
        ResponseBody: From<T>,
    {
        Ok(Response {
            cookies: CookieJar::new(),
            inner: self.inner.body(body.into())?,
        })
    }

    #[inline]
    pub fn json(self, data: &impl Serialize) -> Result<Response, Error> {
        let json = serde_json::to_string(&JsonPayload { data })?;

        self.header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header(CONTENT_LENGTH, json.len())
            .body(json)
    }

    #[inline]
    pub fn html(self, data: impl Into<String>) -> Result<Response, Error> {
        let string = data.into();

        self.header(CONTENT_TYPE, "text/html; charset=utf-8")
            .header(CONTENT_LENGTH, string.len())
            .body(string)
    }

    #[inline]
    pub fn text(self, data: impl Into<String>) -> Result<Response, Error> {
        let string = data.into();

        self.header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(CONTENT_LENGTH, string.len())
            .body(string)
    }

    /// Convert self into a [Response] with an empty payload.
    ///
    #[inline]
    pub fn finish(self) -> Result<Response, Error> {
        self.body(ResponseBody::default())
    }
}
