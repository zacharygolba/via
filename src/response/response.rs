use futures_util::{Stream, StreamExt};
use http::{
    header::{self, HeaderMap},
    StatusCode, Version,
};
use hyper::body::Bytes;

use super::{
    body::{Body, Frame},
    ResponseBuilder,
};
use crate::Error;

pub struct Response {
    pub(super) inner: http::Response<Body>,
}

impl Response {
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn html<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        let body = Body::from(body);
        let len = body.len();

        ResponseBuilder::with_body(Ok(body))
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .headers(len.map(|content_length| (header::CONTENT_LENGTH, content_length)))
    }

    pub fn text<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        let body = Body::from(body);
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
            .map(Body::from)
            .map_err(Error::from);
        let len = body.as_ref().map_or(None, Body::len);

        ResponseBuilder::with_body(body)
            .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .headers(len.map(|content_length| (header::CONTENT_LENGTH, content_length)))
    }

    pub fn stream<T, D, E>(body: T) -> ResponseBuilder
    where
        T: Stream<Item = Result<D, E>> + Send + Sync + 'static,
        Bytes: From<D>,
        Error: From<E>,
    {
        let stream = body.map(|result| match result {
            Ok(data) => Ok(Frame::data(Bytes::from(data))),
            Err(error) => Err(Error::from(error)),
        });

        ResponseBuilder::with_body(Ok(Body::stream(stream)))
            .header(header::TRANSFER_ENCODING, "chunked")
    }

    pub fn body(&self) -> &Body {
        self.inner.body()
    }

    pub fn body_mut(&mut self) -> &mut Body {
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
            inner: http::Response::new(Body::new()),
        }
    }

    pub(crate) fn into_inner(self) -> http::Response<Body> {
        self.inner
    }
}
