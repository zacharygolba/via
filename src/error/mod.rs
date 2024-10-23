mod error;
mod iter;

pub use error::Error;
pub use iter::Iter;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A type alias for [`std::result::Result`] that uses `Error` as the default
/// error type.
///
pub type Result<T> = std::result::Result<T, Error>;
