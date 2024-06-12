#[macro_use]
mod format;

use futures::stream::{BoxStream, StreamExt};
use http::{
    header::{HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    status::{InvalidStatusCode, StatusCode},
};
use http_body_util::{Empty, Full, StreamBody};
use hyper::body::{Body as HyperBody, Bytes};
use std::{
    convert::TryFrom,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{self, Poll},
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{Error, Result};

pub use self::format::*;

type Frame = hyper::body::Frame<Bytes>;
pub(crate) type HyperResponse = http::Response<Body>;

pub enum Body {
    Empty(Empty<Bytes>),
    Full(Full<Bytes>),
    Stream(StreamBody<BoxStream<'static, Result<Frame>>>),
}

pub trait IntoResponse: Sized {
    fn into_response(self) -> Result<Response>;

    fn with_header<K, V>(self, name: K, value: V) -> Result<Response>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        WithHeader::new(self, (name, value)).into_response()
    }

    fn with_status<T>(self, status: T) -> Result<Response>
    where
        StatusCode: TryFrom<T, Error = InvalidStatusCode>,
    {
        WithStatusCode::new(self, status).into_response()
    }
}

#[derive(Default)]
pub struct Response {
    value: http::Response<Body>,
}

pub struct WithHeader<T: IntoResponse> {
    header: Result<(HeaderName, HeaderValue)>,
    value: T,
}

pub struct WithStatusCode<T: IntoResponse> {
    status: Result<StatusCode>,
    value: T,
}

impl Default for Body {
    fn default() -> Self {
        Body::Empty(Empty::default())
    }
}

impl<T> From<T> for Body
where
    T: Into<Full<Bytes>>,
{
    fn from(value: T) -> Self {
        Body::Full(value.into())
    }
}

impl HyperBody for Body {
    type Data = Bytes;
    type Error = Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> Poll<Option<Result<Frame, Self::Error>>> {
        match self.get_mut() {
            Body::Empty(value) => Pin::new(value).poll_frame(cx).map_err(Error::from),
            Body::Full(value) => Pin::new(value).poll_frame(cx).map_err(Error::from),
            Body::Stream(value) => Pin::new(value).poll_frame(cx),
        }
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response> {
        Ok(media!(self, "text/plain"))
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        Ok(media!(self, "text/plain"))
    }
}

impl IntoResponse for File {
    fn into_response(self) -> Result<Response> {
        let stream = StreamBody::new(
            ReaderStream::new(self)
                .map(|result| result.map(Frame::data).map_err(Error::from))
                .boxed(),
        );

        Ok(Response::new(Body::Stream(stream)))
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        let mut response = Response::default();

        *response.status_mut() = StatusCode::NO_CONTENT;
        Ok(response)
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

impl Response {
    pub fn new(body: impl Into<Body>) -> Self {
        Response {
            value: http::Response::new(body.into()),
        }
    }

    pub fn empty() -> Self {
        Response::default()
    }

    pub fn status_code(&self) -> StatusCode {
        self.value.status()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl From<Response> for http::Response<Body> {
    fn from(response: Response) -> Self {
        response.value
    }
}

impl Deref for Response {
    type Target = http::Response<Body>;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for Response {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: IntoResponse> WithHeader<T> {
    fn convert<K, V>(header: (K, V)) -> Result<(HeaderName, HeaderValue)>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        Ok((
            HeaderName::try_from(header.0)?,
            HeaderValue::try_from(header.1)?,
        ))
    }

    fn new<K, V>(value: T, header: (K, V)) -> WithHeader<T>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        WithHeader {
            header: Self::convert(header),
            value,
        }
    }
}

impl<T: IntoResponse> IntoResponse for WithHeader<T> {
    fn into_response(self) -> Result<Response> {
        let mut response = self.value.into_response()?;
        let (name, value) = self.header?;

        response.headers_mut().append(name, value);
        Ok(response)
    }
}

impl<T: IntoResponse> WithStatusCode<T> {
    fn convert<S>(status: S) -> Result<StatusCode>
    where
        StatusCode: TryFrom<S, Error = InvalidStatusCode>,
    {
        Ok(StatusCode::try_from(status)?)
    }

    fn new<S>(value: T, status: S) -> Self
    where
        StatusCode: TryFrom<S, Error = InvalidStatusCode>,
    {
        WithStatusCode {
            status: Self::convert(status),
            value,
        }
    }
}

impl<T: IntoResponse> IntoResponse for WithStatusCode<T> {
    fn into_response(self) -> Result<Response> {
        let mut response = self.value.into_response()?;

        *response.status_mut() = self.status?;
        Ok(response)
    }
}
