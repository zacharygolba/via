use bytes::Bytes;
use cookie::CookieJar;
use http::{Extensions, HeaderMap, StatusCode, Version};
use http_body::Body;
use std::fmt::{self, Debug, Formatter};

use super::body::ResponseBody;
use super::builder::ResponseBuilder;
use crate::error::BoxError;

pub struct Response {
    pub(super) cookies: CookieJar,
    pub(super) inner: http::Response<ResponseBody>,
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
    /// return type of the provided closure.
    ///
    #[inline]
    pub fn map<U, F>(self, map: F) -> Self
    where
        F: FnOnce(ResponseBody) -> U,
        U: Body<Data = Bytes, Error = BoxError> + Send + Sync + 'static,
    {
        Self {
            inner: self.inner.map(|body| body.map(map)),
            ..self
        }
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

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    #[inline]
    pub fn extensions(&self) -> &Extensions {
        self.inner.extensions()
    }

    #[inline]
    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.inner.extensions_mut()
    }

    #[inline]
    pub fn body(&self) -> &ResponseBody {
        self.inner.body()
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
}

impl Response {
    pub(crate) fn cookies_and_headers_mut(&mut self) -> (&mut CookieJar, &mut HeaderMap) {
        let Self { cookies, inner } = self;
        (cookies, inner.headers_mut())
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

impl From<Response> for http::Response<ResponseBody> {
    #[inline]
    fn from(response: Response) -> Self {
        response.inner
    }
}
