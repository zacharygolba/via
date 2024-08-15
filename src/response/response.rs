use bytes::Bytes;
use futures_core::Stream;
use http::{
    header::{self, HeaderMap, CONTENT_TYPE},
    HeaderValue, StatusCode, Version,
};

use super::ResponseBuilder;
use crate::body::{Frame, Pollable, ResponseBody};
use crate::{Error, Result};

pub struct Response {
    pub(super) inner: http::Response<ResponseBody>,
}

impl Response {
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn data_stream<S, D, E>(body: S) -> ResponseBuilder
    where
        S: Stream<Item = Result<D, E>> + Send + 'static,
        D: Into<Bytes> + 'static,
        E: Into<Error> + 'static,
    {
        ResponseBuilder::with_body(Ok(ResponseBody::data_stream(body)))
            .header(header::TRANSFER_ENCODING, "chunked")
    }

    pub fn html<T>(body: T) -> ResponseBuilder
    where
        ResponseBody: From<T>,
    {
        let body = ResponseBody::from(body);
        let len = body.len();

        ResponseBuilder::with_body(Ok(body))
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .headers(len.map(|content_length| (header::CONTENT_LENGTH, content_length)))
    }

    pub fn text<T>(body: T) -> ResponseBuilder
    where
        ResponseBody: From<T>,
    {
        let body = ResponseBody::from(body);
        let len = body.len();

        ResponseBuilder::with_body(Ok(body))
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .headers(len.map(|content_length| (header::CONTENT_LENGTH, content_length)))
    }

    #[cfg(feature = "serde")]
    pub fn json<T>(body: &T) -> ResponseBuilder
    where
        T: serde::Serialize,
    {
        use crate::Error;

        let body = serde_json::to_vec(body)
            .map(ResponseBody::from)
            .map_err(Error::from);
        let len = body.as_ref().map_or(None, ResponseBody::len);

        ResponseBuilder::with_body(body)
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .headers(len.map(|content_length| (header::CONTENT_LENGTH, content_length)))
    }

    pub fn stream<T>(body: T) -> ResponseBuilder
    where
        T: Stream<Item = Result<Frame<Bytes>>> + Send + 'static,
    {
        ResponseBuilder::with_body(Ok(ResponseBody::stream(body)))
            .header(header::TRANSFER_ENCODING, "chunked")
    }

    pub fn map<F>(self, map: F) -> Self
    where
        F: Fn(Bytes) -> Result<Bytes> + Send + Sync + 'static,
    {
        let map = Box::new(map);

        Self {
            inner: self.inner.map(|body| body.map(map)),
        }
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

impl Response {
    pub(crate) fn new() -> Self {
        Self {
            inner: http::Response::new(ResponseBody::new()),
        }
    }

    pub(crate) fn internal_server_error() -> Self {
        let mut response = Response::new();

        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        *response.body_mut() = "Internal Server Error".into();

        let headers = response.headers_mut();

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );

        response
    }

    pub(crate) fn into_inner(self) -> http::Response<Pollable> {
        // Unwrap the inner response and map the body to `Pollable`.
        self.inner.map(ResponseBody::into_pollable)
    }
}
