use cookie::CookieJar;
use http::header::SET_COOKIE;
use http::response::Parts;
use http::{HeaderMap, StatusCode, Version};
use std::fmt::{self, Debug, Formatter};

use super::builder::Builder;
use crate::body::{BoxBody, BufferBody, HttpBody};

pub struct Response {
    cookies: Option<CookieJar>,
    response: http::Response<HttpBody<BufferBody>>,
}

impl Response {
    #[inline]
    pub fn build() -> Builder {
        Default::default()
    }

    #[inline]
    pub fn new(body: HttpBody<BufferBody>) -> Self {
        Self {
            cookies: None,
            response: http::Response::new(body),
        }
    }

    #[inline]
    pub fn from_parts(parts: Parts, body: HttpBody<BufferBody>) -> Self {
        Self {
            cookies: None,
            response: http::Response::from_parts(parts, body),
        }
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the provided closure `map`.
    ///
    #[inline]
    pub fn map(self, map: impl FnOnce(HttpBody<BufferBody>) -> BoxBody) -> Self {
        if cfg!(debug_assertions) && self.body().is_dyn() {
            // TODO: Replace this with tracing and a proper logger.
            eprintln!("calling response.map() more than once can create a reference cycle.");
        }

        Self {
            cookies: self.cookies,
            response: self.response.map(|body| HttpBody::Box(map(body))),
        }
    }

    #[inline]
    pub fn body(&self) -> &HttpBody<BufferBody> {
        self.response.body()
    }

    #[inline]
    pub fn body_mut(&mut self) -> &mut HttpBody<BufferBody> {
        self.response.body_mut()
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
        self.response.headers()
    }

    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.response.headers_mut()
    }

    #[inline]
    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    #[inline]
    pub fn status_mut(&mut self) -> &mut StatusCode {
        self.response.status_mut()
    }

    #[inline]
    pub fn version(&self) -> Version {
        self.response.version()
    }
}

impl Response {
    #[inline]
    pub(crate) fn from_inner(response: http::Response<HttpBody<BufferBody>>) -> Self {
        Self {
            cookies: None,
            response,
        }
    }

    /// Consumes the response and returns the inner value after performing any
    /// final processing that may be required before the response is sent to the
    /// client.
    ///
    pub(crate) fn into_inner(self) -> http::Response<HttpBody<BufferBody>> {
        self.response
    }

    pub(crate) fn set_cookie_headers(&mut self) {
        let cookies = match &self.cookies {
            Some(jar) => jar,
            None => return,
        };

        self.response
            .headers_mut()
            .extend(cookies.delta().filter_map(|cookie| {
                match cookie.encoded().to_string().parse() {
                    Ok(header_value) => Some((SET_COOKIE, header_value)),
                    Err(error) => {
                        let _ = error; // Placeholder for tracing...
                        None
                    }
                }
            }));
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
            .field("body", self.body())
            .finish()
    }
}
