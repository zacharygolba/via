use bytes::{Bytes, BytesMut};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::Error;
use crate::raise;

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Copy the bytes in self into a unique, contiguous `Bytes` instance.
    ///
    fn copy_to_bytes(&mut self) -> Bytes;

    /// Deserialize the payload as an instance of type `T`.
    ///
    /// If type `T` is in a top-level data field of the JSON object located at
    /// the root of the payload, it is automatically resolved.
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use serde::Deserialize;
    /// # use via::Payload;
    /// #
    /// #[derive(Deserialize)]
    /// struct Cat {
    ///     name: String,
    /// }
    ///
    /// let mut payload = Bytes::copy_from_slice(b"{\"data\":{\"name\":\"Ciro\"}}");
    /// let cat = payload.parse_json::<Cat>().expect("invalid payload");
    ///
    /// println!("Meow, {}!", cat.name);
    /// // => Meow, Ciro!
    /// ```
    ///
    fn parse_json<T>(&mut self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        parse_json(&self.copy_to_bytes())
    }

    /// Copy the bytes in self into an owned, contiguous `String`.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    fn to_utf8(&mut self) -> Result<String, Error> {
        String::from_utf8(self.to_vec()).map_err(|error| raise!(400, error))
    }

    /// Copy the bytes in self into a contiguous `Vec<u8>`.
    ///
    fn to_vec(&mut self) -> Vec<u8> {
        self.copy_to_bytes().into()
    }
}

pub fn parse_json<T>(slice: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    #[derive(Deserialize)]
    struct JsonPayload<T> {
        data: T,
    }

    // Attempt deserialize JSON assuming that type `D` exists in a top
    // level data field. This is a common pattern so we optimize for it to
    // provide a more convenient API. If you frequently expect `D` to be at
    // the root of the JSON object contained in `payload` and not in a top-
    // level `data` field, we recommend writing a utility function that
    // circumvents the extra call to deserialize. Otherwise, this has no
    // additional overhead.
    serde_json::from_slice(slice)
        // If `D` was contained in a top-level `data` field, unwrap it.
        .map(|object: JsonPayload<T>| object.data)
        // Otherwise, attempt to deserialize `D` from the object at the
        // root of payload. If that also fails, use the original error.
        .or_else(|error| serde_json::from_slice(slice).or(Err(error)))
        // If an error occurred, wrap it with `via::Error` and set the status
        // code to 400 Bad Request.
        .map_err(|error| raise!(400, error))
}

impl Payload for Bytes {
    fn copy_to_bytes(&mut self) -> Bytes {
        let remaining = self.len();
        let detached = self.split_to(remaining);

        let mut dest = BytesMut::with_capacity(remaining);
        dest.extend_from_slice(detached.as_ref());
        dest.freeze()
    }

    fn to_vec(&mut self) -> Vec<u8> {
        let remaining = self.len();
        let detached = self.split_to(remaining);

        let mut dest = Vec::with_capacity(remaining);
        dest.extend_from_slice(detached.as_ref());
        dest
    }
}
