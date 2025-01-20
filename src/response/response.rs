use cookie::CookieJar;
use http::header::SET_COOKIE;
use http::response::Parts;
use http::{HeaderMap, StatusCode, Version};
use std::fmt::{self, Debug, Formatter};

use super::builder::Builder;
use crate::body::{BoxBody, HttpBody, ResponseBody};

pub struct Response {
    pub(super) cookies: Option<CookieJar>,
    pub(super) inner: http::Response<HttpBody<ResponseBody>>,
}

impl Response {
    #[inline]
    pub fn build() -> Builder {
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
        if cfg!(debug_assertions) && matches!(self.inner.body(), HttpBody::Mapped(_)) {
            // TODO: Replace this with tracing and a proper logger.
            eprintln!("calling response.map() more than once can create a reference cycle.");
        }

        Self {
            cookies: self.cookies,
            inner: self.inner.map(|body| HttpBody::Mapped(map(body))),
        }
    }

    /// Returns a reference to the response cookies.
    ///
    #[inline]
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_ref()
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

impl Response {
    #[inline]
    pub(crate) fn into_inner(self) -> http::Response<HttpBody<ResponseBody>> {
        self.inner
    }

    pub(crate) fn set_cookie_headers(&mut self) {
        let headers = self.inner.headers_mut();
        let delta = match &self.cookies {
            Some(jar) => jar.delta(),
            None => return,
        };

        for cookie in delta {
            let set_cookie = match cookie.encoded().to_string().parse() {
                Ok(header_value) => header_value,
                Err(error) => {
                    let _ = error; // Placeholder for tracing
                    continue;
                }
            };

            if let Err(error) = headers.try_append(SET_COOKIE, set_cookie) {
                let _ = error; // Placeholder for tracing
            }
        }
    }
}

impl Default for Response {
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
