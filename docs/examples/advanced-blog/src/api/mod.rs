pub mod posts;
pub mod users;

use serde::{Deserialize, Serialize};
use via::{
    response::{self, IntoResponse, Response},
    Result,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Document<T> {
    pub data: T,
}

impl<T> Document<T> {
    pub fn new(data: T) -> Document<T> {
        Document { data }
    }
}

impl<T: Serialize> IntoResponse for Document<T> {
    fn into_response(self) -> Result<Response> {
        response::json(&self).into_response()
    }
}
