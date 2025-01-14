#[doc(hidden)]
pub mod allow_method;
pub mod cookie_parser;
pub mod error_boundary;

mod middleware;
mod next;
mod timeout;

pub use allow_method::AllowMethod;
pub use cookie_parser::CookieParser;
pub use middleware::{BoxFuture, Middleware};
pub use next::Next;
pub use timeout::{timeout, Timeout};

pub(crate) use self::middleware::ArcMiddleware;

use crate::Error;

/// Shorthand for a `Result` returned from an async middleware function.
///
pub type Result<T> = std::result::Result<T, Error>;
