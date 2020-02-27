use crate::http::header::{self, HeaderValue};
use crate::{http::StatusCode, Error};
use hyper::body::Body;
use serde::Serialize;

pub type Response = http::Response<Body>;

pub trait Respond {
    fn respond(self) -> Result<Response, Error>;
}

pub struct Json {
    body: Result<Vec<u8>, Error>,
}

#[inline]
pub fn json<T: Serialize>(value: &T) -> Json {
    Json {
        body: serde_json::to_vec(value).map_err(Error::from),
    }
}

impl From<Error> for Response {
    #[inline]
    fn from(error: Error) -> Response {
        if let Some(response) = error.response {
            response
        } else {
            let mut response = Response::new(error.to_string().into());

            *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            response
        }
    }
}

impl Respond for Json {
    #[inline]
    fn respond(self) -> Result<Response, Error> {
        let mut response = Response::new(self.body?.into());

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
