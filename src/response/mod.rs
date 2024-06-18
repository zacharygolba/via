mod body;

use http::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode, Version,
};

use self::body::Body;
use crate::{Error, Result};

pub(crate) type OutgoingResponse = http::Response<Body>;

pub trait IntoResponse: Sized {
    fn into_response(self) -> Result<Response>;
}

pub struct Response {
    inner: OutgoingResponse,
}

pub struct ResponseBuilder {
    body: Option<Result<Body>>,
    inner: http::response::Builder,
}

impl Response {
    pub fn new() -> ResponseBuilder {
        ResponseBuilder {
            body: None,
            inner: http::response::Builder::new(),
        }
    }

    pub fn html<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        Response::with_body(Ok(body.into())).header(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )
    }

    pub fn text<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        Response::with_body(Ok(body.into())).header(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
    }

    #[cfg(feature = "serde")]
    pub fn json<T>(body: &T) -> ResponseBuilder
    where
        T: serde::Serialize,
    {
        let body_bytes = serde_json::to_vec(body)
            .map(Body::from)
            .map_err(Error::from);

        Response::with_body(body_bytes).header(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        )
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

    pub(crate) fn with_body(body: Result<Body>) -> ResponseBuilder {
        ResponseBuilder {
            body: Some(body),
            inner: http::response::Builder::new(),
        }
    }

    pub(crate) fn into_hyper_response(self) -> OutgoingResponse {
        self.inner
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl ResponseBuilder {
    pub fn body<T>(self, body: T) -> Self
    where
        Body: TryFrom<T>,
        <Body as TryFrom<T>>::Error: Into<Error>,
    {
        Self {
            body: Some(Body::try_from(body).map_err(Into::into)),
            inner: self.inner,
        }
    }

    pub fn end(mut self) -> Result<Response> {
        Ok(Response {
            inner: self.inner.body(match self.body.take() {
                Some(body) => body?,
                None => Body::empty(),
            })?,
        })
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self {
            body: self.body,
            inner: self.inner.header(key, value),
        }
    }

    pub fn status<T>(self, status: T) -> Self
    where
        StatusCode: TryFrom<T>,
        <StatusCode as TryFrom<T>>::Error: Into<http::Error>,
    {
        Self {
            body: self.body,
            inner: self.inner.status(status),
        }
    }

    pub fn version(self, version: Version) -> Self {
        Self {
            body: self.body,
            inner: self.inner.version(version),
        }
    }
}

impl IntoResponse for ResponseBuilder {
    fn into_response(self) -> Result<Response> {
        self.end()
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    Error: From<E>,
    T: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        self?.into_response()
    }
}
