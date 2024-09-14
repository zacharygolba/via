use bytes::Bytes;
use futures::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
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
    inner: http::Response<ResponseBody>,
}

impl Response {
    pub fn new(body: ResponseBody) -> Self {
        Self {
            inner: http::Response::new(body),
        }
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
        T: Stream<Item = Result<Frame<Bytes>, E>> + Send + 'static,
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
            inner: http::Response::from_parts(parts, body),
        }
    }

    pub fn into_parts(self) -> (Parts, ResponseBody) {
        self.inner.into_parts()
    }

    /// Consumes the response returning a new response with body mapped to the
    /// return type of the provided closure `map`.
    pub fn map<F, T, E>(self, map: F) -> Self
    where
        F: FnOnce(AnyBody<ByteBuffer>) -> T,
        T: Body<Data = Bytes, Error = E> + Send + 'static,
        Error: From<E>,
    {
        let (parts, body) = self.inner.into_parts();
        let output = map(body.into_inner());
        let body = ResponseBody::from_dyn(output);

        Self::from_parts(parts, body)
    }

    pub fn body(&self) -> &ResponseBody {
        self.inner.body()
    }

    pub fn body_mut(&mut self) -> &mut ResponseBody {
        self.inner.body_mut()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.inner.headers_mut()
    }

    /// A shorthand method for `self.headers_mut().insert(name, value)`.
    ///
    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers_mut().insert(name, value);
    }

    /// A shorthand method for `*self.status_mut() = status`.
    ///
    pub fn set_status(&mut self, status: StatusCode) {
        *self.inner.status_mut() = status;
    }

    pub fn status(&self) -> StatusCode {
        self.inner.status()
    }

    pub fn status_mut(&mut self) -> &mut StatusCode {
        self.inner.status_mut()
    }

    pub fn version(&self) -> Version {
        self.inner.version()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl From<http::Response<ResponseBody>> for Response {
    fn from(inner: http::Response<ResponseBody>) -> Self {
        Self { inner }
    }
}

impl From<Response> for http::Response<AnyBody<ByteBuffer>> {
    fn from(response: Response) -> Self {
        response.inner.map(ResponseBody::into_inner)
    }
}
