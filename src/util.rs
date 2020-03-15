use crate::{http::header, Response};

pub fn is_json(resp: &Response) -> bool {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .map_or(false, |value| value == "application/json")
}
