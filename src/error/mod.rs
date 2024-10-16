mod error;
mod iter;

pub use error::{bad_request, gateway_timeout, internal_server_error, not_found, Error};
pub use iter::Iter;

pub type AnyError = Box<dyn std::error::Error + Send + Sync>;

/// A type alias for [`std::result::Result`] that uses `Error` as the default
/// error type.
///
pub type Result<T, E = Error> = std::result::Result<T, E>;
