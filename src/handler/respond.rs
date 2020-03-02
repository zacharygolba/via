use crate::http::header::{self, HeaderValue};
use crate::{http::StatusCode, Error};
use hyper::body::Body;
use serde::Serialize;

pub type Response = http::Response<Body>;

pub trait Respond: Sized {
    fn respond(self) -> Result<Response, Error>;

    #[inline]
    fn status(self, value: u16) -> Status<Self> {
        Status(value, self)
    }
}

pub struct Json(Result<Vec<u8>, Error>);

pub struct Status<T: Respond>(u16, T);

#[inline]
pub fn json<T: Serialize>(value: &T) -> Json {
    Json(serde_json::to_vec(value).map_err(Error::from))
}

#[inline]
pub fn status(code: u16) -> Status<&'static str> {
    Status(code, "")
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

impl Respond for Response {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        Ok(self)
    }
}

impl Respond for StatusCode {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        let mut response = Response::new(match self.canonical_reason() {
            Some(reason) => reason.into(),
            None => Body::empty(),
        });

        *response.status_mut() = self;
        Ok(response)
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

impl<T: Respond> Respond for Status<T> {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        let Status(code, value) = self;
        let mut response = value.respond()?;

        *response.status_mut() = StatusCode::from_u16(code)?;
        Ok(response)
    }
}
