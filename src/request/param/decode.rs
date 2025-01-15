use percent_encoding::percent_decode_str;
use std::borrow::Cow;

use crate::error::Error;

/// A trait that defines how to decode the value of a parameter.
///
pub trait DecodeParam {
    fn decode(encoded: &str) -> Result<Cow<str>, Error>;
}

/// The default decoder used for unencoded path and query params.
///
pub struct NoopDecode;

/// The decoder used for percent-encoded path and query params.
///
pub struct PercentDecode;

impl DecodeParam for NoopDecode {
    #[inline]
    fn decode(encoded: &str) -> Result<Cow<str>, Error> {
        Ok(Cow::Borrowed(encoded))
    }
}

impl DecodeParam for PercentDecode {
    fn decode(encoded: &str) -> Result<Cow<str>, Error> {
        Ok(percent_decode_str(encoded).decode_utf8()?)
    }
}
