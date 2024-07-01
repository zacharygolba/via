pub mod posts;
pub mod users;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Document<T> {
    pub data: T,
}
