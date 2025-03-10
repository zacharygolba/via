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

fn prefer_unboxed(body: &HttpBody<ResponseBody>) {
    if matches!(body, HttpBody::Dyn(_) | HttpBody::Tee(_)) {
        // TODO: Replace this with tracing and a proper logger.
        eprintln!("boxing a response body more than once can create a reference cycle.");
    }
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
    pub fn map(self, map: impl FnOnce(HttpBody<ResponseBody>) -> BoxBody) -> Self {
        if cfg!(debug_assertions) {
            prefer_unboxed(self.inner.body());
        }

        Self {
            inner: self.inner.map(|body| HttpBody::Dyn(map(body))),
            ..self
        }
    }

    /// Write the contents of the response body into the provided sink when it
    /// is read.
    ///
    #[inline]
    pub fn tee(self, sink: impl AsyncWrite + Send + Sync + 'static) -> Self {
        if cfg!(debug_assertions) {
            prefer_unboxed(self.inner.body());
        }

        Self {
            inner: self.inner.map(|body| {
                HttpBody::Tee(match body {
                    HttpBody::Inline(body) => TeeBody::new(body, sink),
                    HttpBody::Dyn(boxed) => TeeBody::new(boxed, sink),
                    HttpBody::Tee(tee) => TeeBody::new(tee.cap(), sink),
                })
            }),
            ..self
        }
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
