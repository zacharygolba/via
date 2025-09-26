use bytes::{Buf, Bytes};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::borrow::Cow;

use crate::error::Error;

#[derive(Deserialize)]
struct JsonPayload<T> {
    data: T,
}

/// Interact with data received from a client.
///
pub trait Payload: Sized {
    /// Returns a byte slice if the payload is stored contiguously in memory.
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use via::Payload;
    /// #
    /// let payload = Bytes::copy_from_slice(b"hello, world!");
    ///
    /// if let Some(slice) = payload.as_slice() {
    ///     println!("Contiguous: [len: {}].", slice.len());
    /// }
    /// ```
    ///
    fn as_slice(&self) -> Option<&[u8]>;

    /// Copy the bytes in self into an owned, contiguous `Vec<u8>`.
    ///
    /// Any buffers contained in self are advanced to the end and consumed.
    ///
    fn to_vec(self) -> Vec<u8>;

    /// Borrows the payload and converts it to a `UTF-8` string slice if it is
    /// contiguous.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use via::Payload;
    /// #
    /// let payload = Bytes::copy_from_slice(b"hello, world!");
    ///
    /// match payload.as_str() {
    ///     Ok(Some(utf8)) => println!("Contiguous and UTF-8: {}.", utf8),
    ///     Ok(None) => println!("Not contiguous."),
    ///     Err(_) => eprintln!("Invalid UTF-8."),
    /// }
    /// ```
    ///
    fn as_str(&self) -> Result<Option<&str>, Error> {
        self.as_slice()
            .map(str::from_utf8)
            .transpose()
            .map_err(Error::bad_request)
    }

    /// Deserialize the payload as an instance of type `T`.
    ///
    /// If type `T` is in a top-level data field of the JSON object located at
    /// the root of the payload, it is automatically resolved.
    ///
    /// # Example
    ///
    /// ```
    /// # use bytes::Bytes;
    /// # use via::Payload;
    /// # use serde::Deserialize;
    /// #
    /// #[derive(Deserialize)]
    /// struct Cat {
    ///     name: String,
    /// }
    ///
    /// let payload = Bytes::copy_from_slice(b"{\"data\":{\"name\":\"Ciro\"}}");
    /// let cat = payload.json::<Cat>().expect("invalid payload");
    ///
    /// println!("Meow, {}!", cat.name);
    /// // => Meow, Ciro!
    /// ```
    ///
    fn json<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let input = match self.as_str()? {
            Some(str) => Cow::Borrowed(str),
            None => Cow::Owned(self.to_utf8()?),
        };

        // Attempt deserialize JSON assuming that type `D` exists in a top
        // level data field. This is a common pattern so we optimize for it to
        // provide a more convenient API. If you frequently expect `D` to be at
        // the root of the JSON object contained in `payload` and not in a top-
        // level `data` field, we recommend writing a utility function that
        // circumvents the extra call to deserialize. Otherwise, this has no
        // additional overhead.
        serde_json::from_str(input.as_ref())
            // If `D` was contained in a top-level `data` field, unwrap it.
            .map(|object: JsonPayload<T>| object.data)
            // Otherwise, attempt to deserialize `D` from the object at the
            // root of payload. If that also fails, use the original error.
            .or_else(|error| serde_json::from_str(input.as_ref()).or(Err(error)))
            // If an error occurred, wrap it with `via::Error` and set the status
            // code to 400 Bad Request.
            .map_err(Error::bad_request)
    }

    /// Copy the bytes in self into an owned, contiguous `String`.
    ///
    /// Any buffers contained in self are advanced to the end and consumed.
    ///
    /// # Errors
    ///
    /// If the payload is not valid `UTF-8`.
    ///
    fn to_utf8(self) -> Result<String, Error> {
        String::from_utf8(self.to_vec()).map_err(Error::bad_request)
    }
}

impl Payload for Bytes {
    #[inline]
    fn as_slice(&self) -> Option<&[u8]> {
        Some(self.as_ref())
    }

    #[inline]
    fn to_vec(mut self) -> Vec<u8> {
        let remaining = self.remaining();
        let mut vec = Vec::with_capacity(remaining);

        vec.extend_from_slice(&self);
        self.advance(remaining);

        vec
    }
}

#[cfg(feature = "ws")]
impl Payload for bytestring::ByteString {
    #[inline]
    fn as_slice(&self) -> Option<&[u8]> {
        Some(self.as_ref())
    }

    #[inline]
    fn as_str(&self) -> Result<Option<&str>, Error> {
        Ok(Some(self.as_ref()))
    }

    #[inline]
    fn to_vec(self) -> Vec<u8> {
        Payload::to_vec(self.into_bytes())
    }
}
