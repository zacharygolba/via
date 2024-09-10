use bytes::Bytes;
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::response::Parts;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Version};
use http_body::Frame;
use hyper::body::Body;
use serde::Serialize;
use std::fmt::{self, Debug, Formatter};

use super::{ResponseBody, ResponseBuilder, StreamAdapter};
use super::{APPLICATION_JSON, CHUNKED_ENCODING, TEXT_HTML, TEXT_PLAIN};
use crate::body::{AnyBody, Boxed, Buffer};
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
        let buf = body.into_bytes();
        let len = buf.len();

        let mut response = Self::new(ResponseBody::from_vec(buf));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, TEXT_HTML);
        headers.insert(CONTENT_LENGTH, len.into());

        response
    }

    pub fn text(body: String) -> Self {
        let buf = body.into_bytes();
        let len = buf.len();

        let mut response = Self::new(ResponseBody::from_vec(buf));
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, TEXT_PLAIN);
        headers.insert(CONTENT_LENGTH, len.into());

        response
    }

    pub fn json<B: Serialize>(body: &B) -> Result<Response, Error> {
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
    /// If you want to use a stream that is `!Unpin`, you can use the [`Boxed`]
    /// body in combination with a [`ResponseBuilder`]. However, you must ensure
    ///
    pub fn stream<S, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let stream_body = Boxed::new(StreamAdapter::new(stream));
        let mut response = Self::new(ResponseBody::from_boxed(stream_body));

        response.set_header(TRANSFER_ENCODING, CHUNKED_ENCODING);
        response
    }

    pub fn not_found() -> Self {
        let mut response = Response::text("Not Found".to_string());

        *response.status_mut() = StatusCode::NOT_FOUND;
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
    ///
    /// # Errors
    ///
    /// Returns an error if the response body was created with `Pinned`.
    ///
    /// If you need to map an `!Unpin` response body, you must implement the
    /// map logic yourself and pass the result to `Pinned::new`.
    ///
    pub fn map<F, B, E>(self, map: F) -> Result<Self, Error>
    where
        F: FnOnce(AnyBody<Box<Buffer>>) -> B,
        B: Body<Data = Bytes, Error = E> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let (parts, body) = self.inner.into_parts();
        let output = match body.try_into_unpin() {
            Ok(any) => map(any),
            Err(_) => {
                if cfg!(debug_assertions) {
                    //
                    // TODO:
                    //
                    // Replace this with tracing.
                    //
                    eprintln!("mapping a pinned response body is not supported");
                }

                return Err(Error::new("Internal Server Error".to_string()));
            }
        };
        let body = ResponseBody::from_boxed(Boxed::new(output));

        Ok(Self::from_parts(parts, body))
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

impl From<Response> for http::Response<ResponseBody> {
    fn from(response: Response) -> Self {
        response.inner
    }
}
