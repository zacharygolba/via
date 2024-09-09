pub mod posts;
pub mod users;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use via::request::RequestBody;
use via::Error;

#[derive(Debug, Deserialize, Serialize)]
struct Payload<T> {
    data: T,
}

async fn deserialize<D: DeserializeOwned>(body: RequestBody) -> Result<D, Error> {
    // Deserialize the request body into a `Payload<D>`.
    let payload: Payload<D> = body.read_json().await?;

    // Return the `data` field from the JSON request body.
    Ok(payload.data)
}
