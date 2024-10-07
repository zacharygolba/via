use bytes::Bytes;
use cookie::CookieJar;
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, SET_COOKIE, TRANSFER_ENCODING};
use http::response::Parts;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use http_body::{Body, Frame};
use serde::Serialize;
use std::fmt::{self, Debug, Formatter};

use super::{ResponseBody, ResponseBuilder, StreamAdapter};
use super::{APPLICATION_JSON, CHUNKED_ENCODING, TEXT_HTML, TEXT_PLAIN};
use crate::body::{AnyBody, ByteBuffer};
use crate::{Error, Result};

pub struct Response {
    cookies: Option<Box<CookieJar>>,
    response: http::Response<ResponseBody>,
}

impl Response {
    pub fn new(body: ResponseBody) -> Self {
        Self {
            cookies: None,
            response: http::Response::new(body),
        }
    }

    pub fn new_with_status(body: ResponseBody, status: StatusCode) -> Self {
        let mut response = Self::new(body);

        response.set_status(status);
        response
    }

    pub fn html(body: String) -> Self {
        let len = body.len();

        let mut response = Self::new(ResponseBody::from_string(body));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, TEXT_HTML);
        headers.insert(CONTENT_LENGTH, len.into());

        response
    }

    pub fn text(body: String) -> Self {
        let len = body.len();

        let mut response = Self::new(ResponseBody::from_string(body));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, TEXT_PLAIN);
        headers.insert(CONTENT_LENGTH, len.into());

        response
    }

    pub fn json<T: Serialize>(body: &T) -> Result<Response, Error> {
        let buf = serde_json::to_vec(body)?;
        let len = buf.len();

        let mut response = Self::new(ResponseBody::from_vec(buf));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, APPLICATION_JSON);
        headers.insert(CONTENT_LENGTH, len.into());

        Ok(response)
    }

    /// Create a response from a stream of `Result<Frame<Bytes>, Error>`.
    ///
    pub fn stream<T, E>(stream: T) -> Self
    where
        T: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let stream_body = StreamAdapter::new(stream);
        let mut response = Self::new(ResponseBody::from_dyn(stream_body));

        response.set_header(TRANSFER_ENCODING, CHUNKED_ENCODING);
        response
    }

    pub fn not_found() -> Self {
        let mut response = Response::text("Not Found".to_string());

        response.set_status(StatusCode::NOT_FOUND);
        response
    }

    pub fn build() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn from_parts(parts: Parts, body: ResponseBody) -> Self {
        Self {
            cookies: None,
            response: http::Response::from_parts(parts, body),
        }
    }

    /// Consumes the response and returns a tuple containing the component
    /// parts of the response and the response body.
    ///
    pub fn into_parts(self) -> (Parts, ResponseBody) {
        self.response.into_parts()
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the provided closure `map`.
    pub fn map<F, T>(self, map: F) -> Self
    where
        F: FnOnce(AnyBody<ByteBuffer>) -> T,
        T: Body<Data = Bytes, Error = Error> + Send + Sync + 'static,
    {
        let (parts, body) = self.response.into_parts();
        let output = map(body.into_inner());
        let body = ResponseBody::from_dyn(output);

        Self::from_parts(parts, body)
    }

    pub fn body(&self) -> &ResponseBody {
        self.response.body()
    }

    pub fn body_mut(&mut self) -> &mut ResponseBody {
        self.response.body_mut()
    }

    /// Returns a reference to the response cookies.
    ///
    pub fn cookies(&self) -> Option<&CookieJar> {
        self.cookies.as_deref()
    }

    /// Returns a mutable reference to the response cookies.
    ///
    pub fn cookies_mut(&mut self) -> &mut CookieJar {
        self.cookies.get_or_insert_with(Default::default)
    }

    pub fn headers(&self) -> &HeaderMap {
        self.response.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.response.headers_mut()
    }

    /// A shorthand method for `self.headers_mut().insert(name, value)`.
    ///
    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers_mut().insert(name, value);
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn status_mut(&mut self) -> &mut StatusCode {
        self.response.status_mut()
    }

    /// A shorthand method for `*self.status_mut() = status`.
    ///
    pub fn set_status(&mut self, status: StatusCode) {
        *self.response.status_mut() = status;
    }

    pub fn version(&self) -> Version {
        self.response.version()
    }
}

impl Response {
    /// Consumes the response and returns the inner value after performing any
    /// final processing that may be required before the response is sent to the
    /// client.
    ///
    pub(crate) fn into_outgoing_response(self) -> http::Response<AnyBody<ByteBuffer>> {
        // Extract the component parts of the inner response.
        let (mut parts, body) = self.response.into_parts();

        // Map any cookies that have changed to an iter of "Set-Cookie" headers.
        let set_cookie_headers = self.cookies.as_ref().map(|cookies| {
            cookies.delta().filter_map(|cookie| {
                match cookie.encoded().to_string().parse() {
                    Ok(header) => Some((SET_COOKIE, header)),
                    Err(error) => {
                        let _ = error;
                        // Placeholder for tracing...
                        None
                    }
                }
            })
        });

        // Extend the response headers with the "Set-Cookie" headers.
        if let Some(headers) = set_cookie_headers {
            parts.headers.extend(headers);
        }

        // Reconstruct a http::Response from the component parts and response
        // body.
        //
        http::Response::from_parts(parts, body.into_inner())
    }

    pub(crate) fn set_cookies(&mut self, cookies: Box<CookieJar>) {
        self.cookies = Some(cookies);
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.response, f)
    }
}

impl From<http::Response<ResponseBody>> for Response {
    fn from(response: http::Response<ResponseBody>) -> Self {
        Self {
            response,
            cookies: None,
        }
    }
}

impl From<Response> for http::Response<AnyBody<ByteBuffer>> {
    fn from(from: Response) -> Self {
        let (parts, body) = from.into_parts();
        let body = body.into_inner();

        Self::from_parts(parts, body)
    }
}
