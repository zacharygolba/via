use super::{Response, ResponseBody, ResponseBuilder};
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
        let body = ResponseBody::try_from(self)?;
        Response::build().body(body).finish()
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response, Error> {
        Vec::from(self).into_response()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response, Error> {
        Ok(Response::text(self))
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response, Error> {
        self.to_owned().into_response()
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
