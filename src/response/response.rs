use http::{
    header::{HeaderMap, HeaderValue},
    StatusCode, Version,
};

use super::{body::Body, ResponseBuilder};

pub(crate) type OutgoingResponse = http::Response<Body>;

pub struct Response {
    inner: OutgoingResponse,
}

impl Response {
    pub fn build() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    pub fn html<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        ResponseBuilder::with_body(Ok(body.into())).header(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )
    }

    pub fn text<T>(body: T) -> ResponseBuilder
    where
        Body: From<T>,
    {
        ResponseBuilder::with_body(Ok(body.into())).header(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )
    }

    #[cfg(feature = "serde")]
    pub fn json<T>(body: &T) -> ResponseBuilder
    where
        T: serde::Serialize,
    {
        use crate::Error;

        let body_bytes = serde_json::to_vec(body)
            .map(Body::from)
            .map_err(Error::from);

        ResponseBuilder::with_body(body_bytes).header(
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
}

impl Response {
    pub(crate) fn from_inner(inner: OutgoingResponse) -> Self {
        Response { inner }
    }

    pub(crate) fn into_inner(self) -> OutgoingResponse {
        self.inner
    }
}
