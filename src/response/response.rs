use cookie::CookieJar;
use http::response::Parts;
use http::{HeaderMap, StatusCode, Version};
use http_body_util::Either;
use std::fmt::{self, Debug, Formatter};

use crate::body::{BoxBody, BufferBody};

use super::body::ResponseBody;
use super::builder::ResponseBuilder;

#[derive(Default)]
pub struct Response {
    pub(crate) cookies: CookieJar,
    pub(crate) inner: http::Response<ResponseBody>,
}

impl Response {
    #[inline]
    pub fn build() -> ResponseBuilder {
        Default::default()
    }

    #[inline]
    pub fn new(body: ResponseBody) -> Self {
        Self {
            cookies: CookieJar::new(),
            inner: http::Response::new(body),
        }
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the provided closure `map`.
    ///
    #[inline]
    pub fn map<F>(self, map: F) -> Self
    where
        F: FnOnce(Either<BufferBody, BoxBody>) -> Either<BufferBody, BoxBody>,
    {
        Self {
            inner: self.inner.map(|body| ResponseBody {
                inner: map(body.inner),
            }),
            ..self
        }
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    /// Returns a reference to the response cookies.
    ///
    #[inline]
    pub fn cookies(&self) -> &CookieJar {
        &self.cookies
    }

    /// Returns a mutable reference to the response cookies.
    ///
    #[inline]
    pub fn cookies_mut(&mut self) -> &mut CookieJar {
        &mut self.cookies
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

impl From<(Parts, ResponseBody)> for Response {
    fn from((parts, body): (Parts, ResponseBody)) -> Self {
        Self {
            cookies: CookieJar::new(),
            inner: http::Response::from_parts(parts, body),
        }
    }
}
