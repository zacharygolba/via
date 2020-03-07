use crate::{
    http::header::{self, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue},
    Error,
};
use hyper::body::Body;
use serde::Serialize;
use std::convert::{TryFrom, TryInto};

pub type Response = http::Response<Body>;

pub trait Respond: Sized {
    fn respond(self) -> Result<Response, Error>;

    #[inline]
    fn header<K, V>(self, key: K, value: V) -> Header<Self>
    where
        HeaderName: TryFrom<K, Error = InvalidHeaderName>,
        HeaderValue: TryFrom<V, Error = InvalidHeaderValue>,
    {
        Header {
            entry: HeaderName::try_from(key)
                .map_err(Error::from)
                .and_then(|key| Ok((key, HeaderValue::try_from(value)?))),
            value: self,
        }
    }

    #[inline]
    fn status(self, value: u16) -> StatusCode<Self> {
        StatusCode(value, self)
    }
}

pub struct Json(Result<Vec<u8>, Error>);

pub struct Header<T: Respond> {
    entry: Result<(HeaderName, HeaderValue), Error>,
    value: T,
}

pub struct StatusCode<T: Respond>(u16, T);

#[inline]
pub fn json<T: Serialize>(value: &T) -> Json {
    Json(serde_json::to_vec(value).map_err(Error::from))
}

#[inline]
pub fn status(code: u16) -> StatusCode<&'static str> {
    StatusCode(code, "")
}

impl Respond for Json {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        let mut response = Response::new(self.0?.into());

        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        Ok(response)
    }
}

impl<T: Respond> Respond for Header<T> {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        let (key, value) = self.entry?;
        let mut response = self.value.respond()?;

        response.headers_mut().append(key, value);
        Ok(response)
    }
}

impl Respond for Response {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        Ok(self)
    }
}

impl Respond for &'static str {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        Ok(Response::new(self.into()))
    }
}

impl Respond for String {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        Ok(Response::new(self.into()))
    }
}

impl<T: Respond, E> Respond for Result<T, E>
where
    Error: From<E>,
    T: Respond,
{
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        self?.respond()
    }
}

impl<T: Respond> Respond for StatusCode<T> {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        let StatusCode(code, value) = self;
        let mut response = value.respond()?;

        *response.status_mut() = code.try_into()?;
        Ok(response)
    }
}
