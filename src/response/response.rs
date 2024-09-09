use bytes::Bytes;
use futures_core::Stream;
use http::header::{HeaderMap, CONTENT_LENGTH, CONTENT_TYPE, TRANSFER_ENCODING};
use http::{StatusCode, Version};
use http_body::Frame;
use hyper::body::Body;
use serde::Serialize;

use super::{ResponseBody, ResponseBuilder};
use crate::body::{AnyBody, Boxed, Buffered, Either, StreamAdapter};
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
    /// body in combination with a [`ResponseBuilder`]. However, you must ensure
    ///
    pub fn stream<S, E>(stream: S) -> ResponseBuilder
    where
        S: Stream<Item = Result<Frame<Bytes>, E>> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let body = Boxed::new(StreamAdapter::new(stream));
        let builder = ResponseBuilder::with_body(Ok(body.into()));

        builder.header(TRANSFER_ENCODING, "chunked")
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
        F: FnOnce(AnyBody<Buffered>) -> B,
        B: Body<Data = Bytes, Error = E> + Send + Unpin + 'static,
        Error: From<E>,
    {
        let (parts, body) = self.inner.into_parts();
        let output = match body.into_inner() {
            Either::Left(any) => map(any),
            Either::Right(_) => {
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
        let body = ResponseBody::from(Boxed::new(output));

        Ok(Self {
            inner: http::Response::from_parts(parts, body),
        })
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

impl Default for Response {
    fn default() -> Self {
        Self::new(Default::default())
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
