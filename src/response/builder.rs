use bytes::Bytes;
use futures_core::Stream;
use http::{HeaderName, HeaderValue, StatusCode, Version, header};
use http_body::Frame;
use http_body_util::StreamBody;
use http_body_util::combinators::BoxBody;
use serde::Serialize;

use super::body::{Json, ResponseBody};
use super::response::Response;
use crate::error::{BoxError, Error};

/// Define how a type finalizes a [`ResponseBuilder`].
///
/// ```
/// use via::response::{Finalize, Response};
/// use via::{Next, Request};
///
/// async fn echo(request: Request, _: Next) -> via::Result {
///     request.finalize(Response::build().header("X-Powered-By", "Via"))
/// }
/// ```
///
pub trait Finalize {
    fn finalize(self, response: ResponseBuilder) -> Result<Response, Error>;
}

#[derive(Debug, Default)]
pub struct ResponseBuilder {
    response: http::response::Builder,
}

impl ResponseBuilder {
    #[inline]
    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            response: self.response.header(key, value),
        }
    }

    #[inline]
    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            response: self.response.status(status),
        }
    }

    #[inline]
    pub fn version(self, version: Version) -> Self {
        Self {
            response: self.response.version(version),
        }
    }

    #[inline]
    pub fn body(self, body: ResponseBody) -> Result<Response, Error> {
        Ok(self.response.body(body)?.into())
    }

    #[inline]
    pub fn json(self, data: &impl Serialize) -> Result<Response, Error> {
        #[derive(Serialize)]
        struct Tagged<'a, T> {
            data: &'a T,
        }

        Json(Tagged { data }).finalize(self)
    }

    #[inline]
    pub fn html(self, data: impl Into<String>) -> Result<Response, Error> {
        let string = data.into();

        self.header(header::CONTENT_LENGTH, string.len())
            .header(header::CONTENT_TYPE, super::TEXT_HTML)
            .body(string.into())
    }

    #[inline]
    pub fn text(self, data: impl Into<String>) -> Result<Response, Error> {
        let string = data.into();

        self.header(header::CONTENT_LENGTH, string.len())
            .header(header::CONTENT_TYPE, super::TEXT_PLAIN)
            .body(string.into())
    }

    /// Convert self into a [Response] with an empty payload.
    ///
    #[inline]
    pub fn finish(self) -> Result<Response, Error> {
        self.body(ResponseBody::default())
    }
}

impl<T> Finalize for T
where
    T: Stream<Item = Result<Frame<Bytes>, BoxError>> + Send + Sync + 'static,
{
    #[inline]
    fn finalize(self, builder: ResponseBuilder) -> Result<Response, Error> {
        builder
            .header(header::TRANSFER_ENCODING, "chunked")
            .body(BoxBody::new(StreamBody::new(self)).into())
    }
}
