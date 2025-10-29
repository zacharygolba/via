use bytes::{Bytes, BytesMut};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::Error;
use crate::{err, raise};

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Copy the bytes in self into a unique, contiguous `Bytes` instance.
    ///
    fn copy_to_bytes(self) -> Bytes;

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
    fn parse_json<T>(self) -> Result<T, Error>
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
    fn into_utf8(self) -> Result<String, Error> {
        String::from_utf8(self.into_vec()).map_err(|error| err!(400, error))
    }

    /// Copy the bytes in self into a contiguous `Vec<u8>`.
    ///
    fn into_vec(self) -> Vec<u8> {
        self.copy_to_bytes().into()
    }
}

pub fn parse_json<T>(slice: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    use serde_json::value::{Map, Value};
    use serde_json::{from_slice, from_value};

    #[derive(Deserialize)]
    struct Json<D> {
        data: Option<D>,
        #[serde(flatten)]
        rest: Map<String, Value>,
    }

    from_slice(slice)
        .and_then(|Json { data, rest }| data.map_or_else(|| from_value(rest.into()), Ok))
        .or_else(|error| raise!(400, error))
}

impl Payload for Bytes {
    fn copy_to_bytes(mut self) -> Bytes {
        let remaining = self.len();
        let detached = self.split_to(remaining);

        let mut dest = BytesMut::with_capacity(remaining);
        dest.extend_from_slice(detached.as_ref());
        dest.freeze()
    }

    fn into_vec(mut self) -> Vec<u8> {
        let remaining = self.len();
        let detached = self.split_to(remaining);

        let mut dest = Vec::with_capacity(remaining);
        dest.extend_from_slice(detached.as_ref());
        dest
    }
}
