use cookie::CookieJar;
use http::response::Parts;
use http::{HeaderMap, StatusCode, Version};
use std::fmt::{self, Debug, Formatter};
use tokio::io::AsyncWrite;

use super::builder::ResponseBuilder;
use crate::body::{BoxBody, HttpBody, ResponseBody, TeeBody};

pub struct Response {
    pub(crate) cookies: Option<Box<CookieJar>>,
    pub(crate) inner: http::Response<HttpBody<ResponseBody>>,
}

impl Response {
    #[inline]
    pub fn build() -> ResponseBuilder {
        Default::default()
    }

    #[inline]
    pub fn new(body: HttpBody<ResponseBody>) -> Self {
        Self {
            cookies: None,
            inner: http::Response::new(body),
        }
    }

    #[inline]
    pub fn from_parts(parts: Parts, body: HttpBody<ResponseBody>) -> Self {
        Self {
            cookies: None,
            inner: http::Response::from_parts(parts, body),
        }
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the provided closure `map`.
    ///
    #[inline]
    pub fn map<F>(self, map: F) -> Self
    where
        F: FnOnce(HttpBody<ResponseBody>) -> HttpBody<ResponseBody>,
    {
        Self {
            inner: self.inner.map(map),
            ..self
        }
    }

    /// Copies bytes from the request body into the provided sink when it is
    /// read.
    ///
    #[inline]
    pub fn tee(self, sink: impl AsyncWrite + Send + Sync + 'static) -> Self {
        self.map(|body| HttpBody::Boxed(BoxBody::new(TeeBody::new(body.boxed(), sink))))
    }

    /// Returns a reference to the response cookies.
    ///
    #[inline]
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_deref()
    }

    /// Returns a mutable reference to the response cookies.
    ///
    #[inline]
    pub fn cookies_mut(&mut self) -> &mut CookieJar {
        self.cookies.get_or_insert_default()
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    #[inline]
    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    #[inline]
    pub fn status_mut(&mut self) -> &mut StatusCode {
        self.inner.status_mut()
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.inner.version()
    }
}

impl Default for Response {
    #[inline]
    fn default() -> Self {
        Self::new(HttpBody::new())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("version", &self.version())
            .field("status", &self.status())
            .field("headers", self.headers())
            .field("cookies", &self.cookies)
            .field("body", self.inner.body())
            .finish()
    }
}
