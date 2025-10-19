use cookie::CookieJar;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, StatusCode, Version};
use serde::Serialize;

use super::body::ResponseBody;
use super::response::Response;
use crate::Pipe;
use crate::error::Error;

/// Serialize the contained type as an untagged JSON response.
///
/// # Example
/// ```
/// use serde::Serialize;
/// use via::{Pipe, Response};
/// use via::response::Json;
///
/// #[derive(Serialize)]
/// struct Cat {
///     name: String,
/// }
///
/// let ciro = Cat {
///     name: "Ciro".to_owned(),
/// };
///
/// let tagged = Response::build().json(&ciro).unwrap();
/// // => { "data": { "name": "Ciro" } }
///
/// let untagged = Json(&ciro).pipe(Response::build()).unwrap();
/// // => { "name": "Ciro" }
/// ```
///
#[derive(Debug)]
pub struct Json<'a, T>(pub &'a T);

#[derive(Debug, Default)]
pub struct ResponseBuilder {
    inner: http::response::Builder,
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
        #[derive(Serialize)]
        struct Tagged<'a, T> {
            data: &'a T,
        }

        Json(&Tagged { data }).pipe(self)
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

impl<'a, T: Serialize> Pipe for Json<'a, T> {
    #[inline]
    fn pipe(self, response: ResponseBuilder) -> Result<Response, Error> {
        let json = serde_json::to_vec(self.0)?;

        response
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .header(CONTENT_LENGTH, json.len())
            .body(json)
    }
}
