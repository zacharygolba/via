mod convert;
mod format;

use futures::stream::BoxStream;
use http::status::StatusCode;
use http_body_util::{Empty, Full, StreamBody};
use hyper::body::{Body as HyperBody, Bytes};
use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{self, Poll},
};

use crate::{Error, Result};

pub use convert::IntoResponse;

#[cfg(feature = "serde")]
pub use format::{json, Json};

type Frame = hyper::body::Frame<Bytes>;
pub(crate) type HyperResponse = http::Response<Body>;

pub enum Body {
    Empty(Empty<Bytes>),
    Full(Full<Bytes>),
    Stream(StreamBody<BoxStream<'static, Result<Frame>>>),
}

#[derive(Default)]
pub struct Response {
    value: http::Response<Body>,
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
