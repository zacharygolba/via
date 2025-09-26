use percent_encoding::percent_decode_str;
use std::borrow::Cow;

use crate::error::Error;

#[derive(Clone, Copy, Debug)]
pub enum UriEncoding {
    Percent,
    Unencoded,
}

impl UriEncoding {
    #[inline]
    pub fn decode<'a>(&self, input: &'a str) -> Result<Cow<'a, str>, Error> {
        if matches!(self, Self::Unencoded) {
            return Ok(Cow::Borrowed(input));
        }

        percent_decode_str(input)
            .decode_utf8()
            .map_err(Error::bad_request)
    }
}
