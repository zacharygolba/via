use std::fmt::{self, Display, Formatter};

use crate::error::{DynError, Error};

#[derive(Clone, Copy, Debug)]
pub struct LimitError;

/// Wrap the provided [BoxError] with [Error] and set the status based on the
/// error type.
///
pub fn error_from_boxed(error: DynError) -> Error {
    if let Some(&LimitError) = error.downcast_ref() {
        Error::payload_too_large(error)
    } else {
        Error::bad_request(error)
    }
}

impl std::error::Error for LimitError {}

impl Display for LimitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "maximum request body size exceeded")
    }
}
