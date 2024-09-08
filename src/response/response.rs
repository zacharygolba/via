use bytes::Bytes;
use futures_core::Stream;
use http::header::{HeaderMap, CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::{StatusCode, Version};
use hyper::body::Body;
use serde::Serialize;

use super::ResponseBuilder;
use crate::body::{AnyBody, Boxed, Buffered, Frame, StreamAdapter};
use crate::{Error, Result};

pub struct Response {
    inner: http::Response<AnyBody<Buffered>>,
}

impl Response {
    pub fn new(body: AnyBody<Buffered>) -> Self {
        Self {
            inner: http::Response::new(body),
        }
    }

    pub fn build() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn html<B>(body: B) -> ResponseBuilder
    where
        Buffered: From<B>,
    {
        let builder = ResponseBuilder::buffered(body);
        builder.header(CONTENT_TYPE, "text/html; charset=utf-8")
    }

    pub fn text<B>(body: B) -> ResponseBuilder
    where
        Buffered: From<B>,
    {
        let builder = ResponseBuilder::buffered(body);
        builder.header(CONTENT_TYPE, "text/plain; charset=utf-8")
    }

    pub fn json<B: Serialize>(body: &B) -> ResponseBuilder {
        let (len, body) = match serde_json::to_vec(body) {
            Ok(data) => (Some(data.len()), Ok(data.into())),
            Err(error) => (None, Err(error.into())),
        };

        ResponseBuilder::with_body(body)
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .headers(Some(CONTENT_LENGTH).zip(len))
    }

    /// Build a response from a stream of `Result<Frame<Bytes>, Error>`.
    ///
    /// If you want to use a stream that is `!Unpin`, you can use the [`Boxed`]
    /// body in combination with a [`ResponseBuilder`].
    ///
    pub fn stream<S, E>(stream: S) -> ResponseBuilder
    where
        S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin + 'static,
        E: Into<Error>,
    {
        let body = Boxed::new(Box::new(StreamAdapter::new(stream)));
        let builder = ResponseBuilder::with_body(Ok(AnyBody::Boxed(body)));

        builder.header(TRANSFER_ENCODING, "chunked")
    }

    pub fn map<F, B>(self, f: F) -> Self
    where
        F: FnOnce(AnyBody<Buffered>) -> B,
        B: Body<Data = Bytes, Error = Error> + Send + 'static,
    {
        let inner = self.inner.map(|input| {
            let output = f(input);
            let body = Boxed::new(Box::new(output));

            AnyBody::Boxed(body)
        });

        Self { inner }
    }

    pub fn body(&self) -> &AnyBody<Buffered> {
        self.inner.body()
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

impl Default for Response {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl From<http::Response<AnyBody<Buffered>>> for Response {
    fn from(inner: http::Response<AnyBody<Buffered>>) -> Self {
        Self { inner }
    }
}

impl From<Response> for http::Response<AnyBody<Buffered>> {
    fn from(response: Response) -> Self {
        response.inner
    }
}
