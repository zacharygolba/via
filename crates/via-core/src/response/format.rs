use super::{Body, IntoResponse, Response};
use crate::Result;

struct Json(Result<Body>);

pub fn json(body: &impl serde::Serialize) -> impl IntoResponse {
    Json(match serde_json::to_vec(body) {
        Ok(bytes) => Ok(bytes.into()),
        Err(error) => Err(error.into()),
    })
}

macro_rules! media(($body:expr, $type:expr) => {{
    use http::header::{CONTENT_TYPE, HeaderValue};

    let mut response = Response::new($body);
    let headers = response.headers_mut();

    headers.insert(CONTENT_TYPE, HeaderValue::from_static($type));
    response
}});

impl IntoResponse for Json {
    fn into_response(self) -> Result<Response> {
        Ok(media!(self.0?, "application/json"))
    }
}
