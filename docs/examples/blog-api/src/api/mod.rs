pub mod posts;
pub mod users;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Payload<T> {
    data: T,
}

impl<T: Serialize> Payload<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}
