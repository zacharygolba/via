use crate::Result;
use http::{
    header::{HeaderName, InvalidHeaderName, InvalidHeaderValue},
    HeaderValue, StatusCode,
};
use hyper::Body;
use serde::Serialize;
use std::convert::{TryFrom, TryInto};

#[doc(hidden)]
pub use serde_json::json as jsonlit;

pub type Response = http::Response<Body>;

pub trait Respond: Sized {
    fn respond(self) -> Result<Response>;

    #[inline]
    fn header<K, V>(self, key: K, value: V) -> Header<Self>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        Header {
            chain: self,
            entry: Header::<Self>::entry(key, value),
        }
    }

    #[inline]
    fn status(self, value: u16) -> Status<Self> {
        Status { chain: self, value }
    }
}

#[non_exhaustive]
pub enum Format {
    Json(Result<Body>),
    Html(Body),
}

pub struct Header<T: Respond> {
    chain: T,
    entry: Result<(HeaderName, HeaderValue)>,
}

pub struct Status<T: Respond> {
    chain: T,
    value: u16,
}

#[inline]
pub fn html(body: impl Into<String>) -> Format {
    Format::Html(Body::from(body.into()))
}

#[inline]
pub fn json(body: &impl Serialize) -> Format {
    Format::Json(match serde_json::to_vec(body) {
        Ok(bytes) => Ok(bytes.into()),
        Err(e) => Err(e.into()),
    })
}

#[macro_export(local_inner_macros)]
macro_rules! json {
    { $($tokens:tt)+ } => {
        $crate::respond::json(&$crate::respond::jsonlit!($($tokens)+))
    };
}

macro_rules! media {
    ($body:expr, $type:expr) => {{
        use http::header::CONTENT_TYPE;

        let mut response = Response::new($body);
        let headers = response.headers_mut();

        headers.insert(CONTENT_TYPE, HeaderValue::from_static($type));
        response
    }};
}

impl Respond for &'static str {
    #[inline]
    fn respond(self) -> Result<Response> {
        Ok(media!(self.into(), "text/plain"))
    }
}

impl Respond for String {
    #[inline]
    fn respond(self) -> Result<Response> {
        Ok(media!(self.into(), "text/plain"))
    }
}

impl Respond for () {
    #[inline]
    fn respond(self) -> Result<Response> {
        let mut response = Response::default();

        *response.status_mut() = StatusCode::NO_CONTENT;
        Ok(response)
    }
}

impl Respond for Format {
    #[inline]
    fn respond(self) -> Result<Response> {
        Ok(match self {
            Format::Html(body) => media!(body, "text/html"),
            Format::Json(body) => media!(body?, "application/json"),
        })
    }
}

impl<T: Respond> Header<T> {
    #[inline]
    fn entry<K, V>(key: K, value: V) -> Result<(HeaderName, HeaderValue)>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        let key = HeaderName::try_from(key)?;
        let value = HeaderValue::try_from(value)?;

        Ok((key, value))
    }
}

impl<T: Respond> Respond for Header<T> {
    #[inline]
    fn respond(self) -> Result<Response> {
        let mut response = self.chain.respond()?;
        let (key, value) = self.entry?;

        response.headers_mut().append(key, value);
        Ok(response)
    }
}

impl Respond for Response {
    #[inline]
    fn respond(self) -> Result<Response> {
        Ok(self)
    }
}

impl<T: Respond> Respond for Result<T> {
    #[inline]
    fn respond(self) -> Result<Response> {
        self?.respond()
    }
}

impl<T: Respond> Respond for Status<T> {
    #[inline]
    fn respond(self) -> Result<Response> {
        let mut response = self.chain.respond()?;
        let status = self.value.try_into()?;

        *response.status_mut() = status;
        Ok(response)
    }
}
