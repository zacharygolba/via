use http::header::{HeaderValue, CONTENT_TYPE};
use serde::Serialize;
use serde_json::Value;

use super::{Body, IntoResponse, Response};
use crate::Result;

pub struct Json(Result<Body>);

pub fn json<T: Serialize>(body: &T) -> Json {
    Json(match serde_json::to_vec(body) {
        Ok(bytes) => Ok(bytes.into()),
        Err(error) => Err(error.into()),
    })
}

impl IntoResponse for Json {
    fn into_response(self) -> Result<Response> {
        let mut response = Response::new(self.0?);

        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Ok(response)
    }
}

impl IntoResponse for Value {
    fn into_response(self) -> Result<Response> {
        json(&self).into_response()
    }
}
