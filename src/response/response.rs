use cookie::{Cookie, CookieJar};
use http::{header::SET_COOKIE, HeaderMap, StatusCode, Version};
use http_body_util::Either;
use std::fmt::{self, Debug, Formatter};

use super::builder::ResponseBuilder;
use crate::body::{BoxBody, BufferBody};
use crate::error::Error;

pub type ResponseBody = Either<BufferBody, BoxBody>;

pub struct Response {
    cookies: CookieJar,
    inner: http::Response<ResponseBody>,
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
        F: FnOnce(ResponseBody) -> ResponseBody,
    {
        Self {
            inner: self.inner.map(map),
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

impl Response {
    pub(crate) fn set_cookies<F>(&mut self, f: F) -> Result<(), Error>
    where
        F: Fn(&Cookie) -> String,
    {
        let headers = self.inner.headers_mut();

        for cookie in self.cookies.delta() {
            headers.try_append(SET_COOKIE, f(cookie).parse()?)?;
        }

        Ok(())
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

impl From<http::Response<ResponseBody>> for Response {
    #[inline]
    fn from(inner: http::Response<ResponseBody>) -> Self {
        Self {
            cookies: CookieJar::new(),
            inner,
        }
    }
}

impl From<Response> for http::Response<ResponseBody> {
    #[inline]
    fn from(response: Response) -> Self {
        response.inner
    }
}
