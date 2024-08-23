use percent_encoding::percent_decode_str;
use std::borrow::Cow;

use crate::error::Error;

/// A trait that defines how to decode the value of a `Param`.
pub trait DecodeParam {
    fn decode(encoded: &str) -> Result<Cow<str>, Error>;
}

/// A decoder that does nothing. This is used in cases where we know the value
/// of a `Param` is already in the correct format.
pub struct NoopDecoder;

/// A decoder that decodes a percent-encoded `&str`.
pub struct PercentDecoder;

impl DecodeParam for NoopDecoder {
    #[inline]
    fn decode(encoded: &str) -> Result<Cow<str>, Error> {
        Ok(Cow::Borrowed(encoded))
    }
}

impl DecodeParam for PercentDecoder {
    fn decode(encoded: &str) -> Result<Cow<str>, Error> {
        Ok(percent_decode_str(encoded).decode_utf8()?)
    }
}
