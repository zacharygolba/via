use std::fmt::{self, Display, Formatter};

use crate::error::{BoxError, Error};

#[derive(Clone, Copy, Debug)]
pub struct LengthLimitError;

/// Wrap the provided [BoxError] with [Error] and set the status based on the
/// error type.
///
pub fn error_from_boxed(error: BoxError) -> Error {
    if let Some(&LengthLimitError) = error.downcast_ref() {
        Error::payload_too_large(error)
    } else {
        Error::bad_request(error)
    }
}

impl std::error::Error for LengthLimitError {}

impl Display for LengthLimitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "maximum request body size exceeded")
    }
}
