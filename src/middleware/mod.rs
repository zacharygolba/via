#[doc(hidden)]
pub mod allow_method;
pub mod error_boundary;

mod middleware;
mod next;
mod timeout;

pub use allow_method::AllowMethod;
pub use error_boundary::ErrorBoundary;
pub use middleware::{BoxFuture, Middleware};
pub use next::Next;
pub use timeout::{timeout, Timeout};

pub(crate) use self::middleware::ArcMiddleware;
