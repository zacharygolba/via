use bytes::Bytes;
use cookie::{Cookie, CookieJar};
use http::header::SET_COOKIE;
use http::{Extensions, HeaderMap, StatusCode, Version};
use http_body::Body;
use std::fmt::{self, Debug, Formatter};

use super::body::ResponseBody;
use super::builder::ResponseBuilder;
use crate::error::{BoxError, Error};

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

impl From<Response> for http::Response<ResponseBody> {
    #[inline]
    fn from(response: Response) -> Self {
        response.inner
    }
}
