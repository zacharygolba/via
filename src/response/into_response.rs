use http::header::{CONTENT_LENGTH, CONTENT_TYPE};

use super::Response;
use crate::body::{BufferBody, HttpBody};
use crate::Error;

pub trait IntoResponse {
    fn into_response(self) -> Result<Response, Error>;
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response, Error> {
        Response::build().finish()
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response, Error> {
        let body = BufferBody::from(self);

        Response::build()
            .header(CONTENT_TYPE, "application/octet-stream")
            .header(CONTENT_LENGTH, body.len())
            .body(HttpBody::Inline(body))
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response, Error> {
        self.to_vec().into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response, Error> {
        Response::build().text(self)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response, Error> {
        Response::build().text(self.to_owned())
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response, Error> {
        Ok(self)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    Error: From<E>,
{
    fn into_response(self) -> Result<Response, Error> {
        self?.into_response()
    }
}
