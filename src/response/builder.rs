use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, StatusCode, Version};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use super::Response;
use crate::body::{HttpBody, ResponseBody};
use crate::error::Error;

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
    pub fn body(self, data: HttpBody<ResponseBody>) -> Result<Response, Error> {
        Ok(Response {
            cookies: None,
            inner: self.inner.body(data)?,
        })
    }

    pub fn json(self, data: &impl Serialize) -> Result<Response, Error> {
        let json = serde_json::to_string(&JsonPayload { data })?;

        self.header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header(CONTENT_LENGTH, json.len())
            .body(HttpBody::Original(ResponseBody::from(json)))
    }

    pub fn html(self, data: String) -> Result<Response, Error> {
        self.header(CONTENT_TYPE, "text/html; charset=utf-8")
            .header(CONTENT_LENGTH, data.len())
            .body(HttpBody::Original(ResponseBody::from(data)))
    }

    pub fn text(self, data: String) -> Result<Response, Error> {
        self.header(CONTENT_TYPE, "text/plain; charset=utf-8")
            .header(CONTENT_LENGTH, data.len())
            .body(HttpBody::Original(ResponseBody::from(data)))
    }

    /// Convert self into a [Response] with an empty payload.
    ///
    #[inline]
    pub fn finish(self) -> Result<Response, Error> {
        self.body(Default::default())
    }
}

impl<T: Serialize> Serialize for JsonPayload<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("JsonPayload", 1)?;

        state.serialize_field("data", self.data)?;
        state.end()
    }
}
