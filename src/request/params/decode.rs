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
///
/// Instead of returning an error if the `&str` contains Invalid UTF-8
/// percent-encoded byte sequences, this decoder will replace the invalid
/// sequences with the Unicode replacement character.
pub struct PercentDecoder;

impl DecodeParam for NoopDecoder {
    #[inline]
    fn decode(encoded: &str) -> Result<Cow<str>, Error> {
        Ok(Cow::Borrowed(encoded))
    }
}

impl DecodeParam for PercentDecoder {
    fn decode(encoded: &str) -> Result<Cow<str>, Error> {
        percent_decode_str(encoded).decode_utf8().or_else(|_| {
            // The decoder encountered an invalid UTF-8 byte sequence.
            //
            // TODO:
            //
            // Implement tracing and include information about the invalid
            // byte sequence that we encountered.

            // Fallback to a lossy decoding strategy. Invalid byte sequences
            // will be replaced with the Unicode replacement character.
            Ok(percent_decode_str(encoded).decode_utf8_lossy())
        })
    }
}
