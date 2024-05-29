#[macro_use]
mod format;

use futures::{
    stream::{self, BoxStream, Stream, StreamExt},
    TryStreamExt,
};
use http::{
    header::{HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    status::{InvalidStatusCode, StatusCode},
};
use http_body_util::{Empty, Full, StreamBody};
use hyper::{
    body::{Body as HyperBody, Bytes},
    Error as HyperError,
};
use std::{
    convert::{Infallible, TryFrom},
    io,
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{self, Poll},
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{Error, Result};

pub use self::format::*;

type Frame = hyper::body::Frame<Bytes>;

pub enum Body {
    Complete(Full<Bytes>),
    Stream(StreamBody<BoxStream<'static, Result<Frame>>>),
    Empty(Empty<Bytes>),
}

pub trait Respond: Sized {
    fn respond(self) -> Result<Response>;

    fn header<K, V>(self, name: K, value: V) -> WithHeader<Self>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        WithHeader::new(self, (name, value))
    }

    fn status<T>(self, status: T) -> WithStatusCode<Self>
    where
        StatusCode: TryFrom<T, Error = InvalidStatusCode>,
    {
        WithStatusCode::new(self, status)
    }
}

#[derive(Default)]
pub struct Response {
    value: http::Response<Body>,
}

pub struct WithHeader<T: Respond> {
    header: Result<(HeaderName, HeaderValue)>,
    value: T,
}

pub struct WithStatusCode<T: Respond> {
    status: Result<StatusCode>,
    value: T,
}

impl Body {
    fn stream<S, V, E>(stream: S) -> Self
    where
        S: Stream<Item = Result<V, E>> + Send + 'static,
        Bytes: From<V>,
        Error: From<E>,
    {
        unimplemented!()
    }
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
        Body::Complete(value.into())
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
            Body::Complete(value) => Pin::new(value).poll_frame(cx).map_err(Error::from),
            Body::Stream(value) => Pin::new(value).poll_frame(cx),
            Body::Empty(value) => Pin::new(value).poll_frame(cx).map_err(Error::from),
        }
    }
}

impl Respond for &'static str {
    fn respond(self) -> Result<Response> {
        Ok(media!(self, "text/plain"))
    }
}

impl Respond for String {
    fn respond(self) -> Result<Response> {
        Ok(media!(self, "text/plain"))
    }
}

impl Respond for File {
    fn respond(self) -> Result<Response> {
        let stream = StreamBody::new(
            ReaderStream::new(self)
                .map(|result| result.map(Frame::data).map_err(Error::from))
                .boxed(),
        );

        Ok(Response::new(Body::Stream(stream)))
    }
}

impl Respond for () {
    fn respond(self) -> Result<Response> {
        let mut response = Response::default();

        *response.status_mut() = StatusCode::NO_CONTENT;
        Ok(response)
    }
}

impl<T, E> Respond for Result<T, E>
where
    Error: From<E>,
    T: Respond,
{
    fn respond(self) -> Result<Response> {
        self?.respond()
    }
}

impl Response {
    pub fn new(body: impl Into<Body>) -> Response {
        Response {
            value: http::Response::new(body.into()),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        self.value.status()
    }
}

impl Respond for Response {
    fn respond(self) -> Result<Response> {
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

impl<T: Respond> WithHeader<T> {
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

impl<T: Respond> Respond for WithHeader<T> {
    fn respond(self) -> Result<Response> {
        let mut response = self.value.respond()?;
        let (name, value) = self.header?;

        response.headers_mut().append(name, value);
        Ok(response)
    }
}

impl<T: Respond> WithStatusCode<T> {
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

impl<T: Respond> Respond for WithStatusCode<T> {
    fn respond(self) -> Result<Response> {
        let mut response = self.value.respond()?;

        *response.status_mut() = self.status?;
        Ok(response)
    }
}

impl Respond for serde_json::Value {
    fn respond(self) -> Result<Response> {
        json(&self).respond()
    }
}
