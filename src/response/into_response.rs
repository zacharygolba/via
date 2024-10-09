use super::{Response, ResponseBody, ResponseBuilder};
use crate::{Error, Result};

pub trait IntoResponse {
    fn into_response(self) -> Result<Response>;
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        Ok(Default::default())
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response> {
        let body = ResponseBody::try_from(self)?;
        Response::build().body(body).finish()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response> {
        Vec::from(self).into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        Ok(Response::text(self))
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response> {
        self.to_string().into_response()
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

impl<T, E> IntoResponse for Result<T, E>
where
    T: IntoResponse,
    Error: From<E>,
{
    fn into_response(self) -> Result<Response> {
        self?.into_response()
    }
}
