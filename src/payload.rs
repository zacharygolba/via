use bytes::{Bytes, BytesMut};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{Error, raise};

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Copy the bytes in self into a unique, contiguous `Bytes` instance.
    ///
    fn copy_to_bytes(self) -> Bytes;

    /// Deserialize and extract `T` as JSON from the top-level data field of
    /// the object contained by the bytes in self.
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
        #[derive(Deserialize)]
        struct Json<D> {
            data: D,
        }

        self.parse_json_untagged().map(|Json { data }| data)
    }

    /// Deserialize type `T` as JSON from the bytes in self.
    ///
    /// The `_untagged` suffix comes from the container or variant attribute
    /// that can be used when deriving `Deserialize` for an enum with the
    /// `serde` crate.
    ///
    /// For additional context as to what a "tag" means and it's releationship
    /// to deserializing JSON, consider reading the following section on enum
    /// representations in the serde docs:
    /// https://serde.rs/enum-representations.html
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
    /// let mut payload = Bytes::copy_from_slice(b"{\"name\":\"Ciro\"}");
    /// let cat = payload.parse_json_untagged::<Cat>().expect("invalid payload");
    ///
    /// println!("Meow, {}!", cat.name);
    /// // => Meow, Ciro!
    /// ```
    ///
    fn parse_json_untagged<T>(self) -> Result<T, Error>
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
        String::from_utf8(self.into_vec()).or_else(|error| raise!(400, error))
    }

    /// Copy the bytes in self into a contiguous `Vec<u8>`.
    ///
    fn into_vec(self) -> Vec<u8> {
        self.copy_to_bytes().into()
    }
}

#[inline]
pub fn parse_json<T>(slice: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    serde_json::from_slice(slice).or_else(|error| raise!(400, error))
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
