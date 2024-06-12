#[cfg(feature = "serde")]
pub mod json;

#[cfg(feature = "serde")]
pub use json::*;

use super::{Body, IntoResponse, Response};

macro_rules! media(($body:expr, $type:expr) => {{
    use http::header::{CONTENT_TYPE, HeaderValue};

    let mut response = Response::new($body);
    let headers = response.headers_mut();

    headers.insert(CONTENT_TYPE, HeaderValue::from_static($type));
    response
}});
