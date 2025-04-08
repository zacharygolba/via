use http_body_util::LengthLimitError;

use crate::error::{DynError, Error};

/// Wrap the provided [BoxError] with [Error] and set the status based on the
/// error type.
///
pub fn map_err(error: DynError) -> Error {
    if error.is::<LengthLimitError>() {
        Error::payload_too_large(error)
    } else {
        Error::bad_request(error)
    }
}
