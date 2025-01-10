use http::header::CONTENT_LENGTH;

use super::{Response, ResponseBuilder};
use crate::body::{BufferBody, HttpBody};
use crate::Error;

pub trait IntoResponse {
    fn into_response(self) -> Result<Response, Error>;
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response, Error> {
        Ok(Default::default())
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response, Error> {
        let body = BufferBody::from(self);
        let len = body.len();

        Response::build()
            .body(HttpBody::Inline(body))
            .header(CONTENT_LENGTH, len)
            .finish()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response, Error> {
        let body = BufferBody::new(self);
        let len = body.len();

        Response::build()
            .body(HttpBody::Inline(body))
            .header(CONTENT_LENGTH, len)
            .finish()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response, Error> {
        Ok(Response::text(self))
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response, Error> {
        let body = BufferBody::new(self.as_bytes());
        let len = body.len();

        Response::build()
            .body(HttpBody::Inline(body))
            .header(CONTENT_LENGTH, len)
            .finish()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response, Error> {
        Ok(self)
    }
}

impl IntoResponse for ResponseBuilder {
    fn into_response(self) -> Result<Response, Error> {
        self.finish()
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
