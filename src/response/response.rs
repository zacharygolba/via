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

pub(crate) type OutgoingResponse = http::Response<Body>;

pub struct Response {
    pub(super) inner: OutgoingResponse,
}

impl Response {
    pub fn build() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn html<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        let body = Body::from(body);
        let content_len = body.len();
        let mut response = ResponseBuilder::with_body(Ok(body));

        response = response.header(header::CONTENT_TYPE, "text/html; charset=utf-8");

        if let Some(value) = content_len {
            response = response.header(header::CONTENT_LENGTH, value)
        }

        response
    }

    pub fn text<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        let body = Body::from(body);
        let content_len = body.len();
        let mut response = ResponseBuilder::with_body(Ok(body));

        response = response.header(header::CONTENT_TYPE, "text/plain; charset=utf-8");

        if let Some(value) = content_len {
            response = response.header(header::CONTENT_LENGTH, value)
        }

        response
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
        let content_len = body.as_ref().map_or(None, Body::len);
        let mut response = ResponseBuilder::with_body(body);

        response = response.header(header::CONTENT_TYPE, "application/json; charset=utf-8");

        if let Some(value) = content_len {
            response = response.header(header::CONTENT_LENGTH, value)
        }

        response
    }

    pub fn stream<T, D, E>(body: T) -> ResponseBuilder
    where
        T: Stream<Item = Result<D, E>> + Send + Sync + 'static,
        Bytes: From<D>,
        Error: From<E>,
    {
        let body = Body::stream(Box::pin(body.map(|result| match result {
            Ok(data) => Ok(Frame::data(Bytes::from(data))),
            Err(error) => Err(Error::from(error)),
        })));

        ResponseBuilder::with_body(Ok(body)).header(header::TRANSFER_ENCODING, "chunked")
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
        Response {
            inner: http::Response::new(Body::empty()),
        }
    }

    pub(crate) fn into_inner(self) -> OutgoingResponse {
        self.inner
    }
}
