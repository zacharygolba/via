pub mod posts;
pub mod users;
pub mod util;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use via::request::RequestBody;

#[derive(Debug, Deserialize, Serialize)]
struct Payload<T> {
    data: T,
}

async fn deserialize<D: DeserializeOwned>(body: RequestBody) -> via::Result<D> {
    // Deserialize the request body into a `Payload<D>`.
    let payload: Payload<D> = body.json().await?;

    // Return the `data` field from the JSON request body.
    Ok(payload.data)
}
