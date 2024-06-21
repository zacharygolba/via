use super::{Response, ResponseBuilder};
use crate::{Error, Result};

pub trait IntoResponse: Sized {
    fn into_response(self) -> Result<Response>;
}

impl IntoResponse for ResponseBuilder {
    fn into_response(self) -> Result<Response> {
        self.end()
    }
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl<T, E> IntoResponse for Result<T, E>
where
    Error: From<E>,
    T: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        self?.into_response()
    }
}
