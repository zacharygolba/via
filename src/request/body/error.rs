use std::fmt::{self, Display, Formatter};

use crate::error::{BoxError, Error};

#[derive(Clone, Copy, Debug)]
pub struct PayloadTooLargeError;

/// Wrap the provided [BoxError] with [Error] and set the status based on the
/// error type.
///
pub fn error_from_boxed(error: BoxError) -> Error {
    if let Some(&PayloadTooLargeError) = error.downcast_ref() {
        Error::payload_too_large(error)
    } else {
        Error::bad_request(error)
    }
}

impl std::error::Error for PayloadTooLargeError {}

impl Display for PayloadTooLargeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Payload Too Large")
    }
}
