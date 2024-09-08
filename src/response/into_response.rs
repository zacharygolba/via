use super::{Response, ResponseBuilder};
use crate::body::{AnyBody, Buffered};
use crate::{Error, Result};

pub trait IntoResponse {
    fn into_response(self) -> Result<Response>;
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        Ok(Response::default())
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response> {
        Response::build().body(self).finish()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response> {
        Response::build().body(self).finish()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        Response::text(self).finish()
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response> {
        Response::text(self).finish()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl IntoResponse for ResponseBuilder {
    fn into_response(self) -> Result<Response> {
        self.finish()
    }
}

impl<B> IntoResponse for http::Response<B>
where
    AnyBody<Buffered>: From<B>,
{
    fn into_response(self) -> Result<Response> {
        Ok(self.map(|body| body.into()).into())
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    Error: From<E>,
{
    fn into_response(self) -> Result<Response> {
        self?.into_response()
    }
}
