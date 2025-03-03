pub mod cookie_parser;
pub mod error_boundary;

pub(crate) mod accept_method;

#[cfg(feature = "fs")]
mod favicon;

mod middleware;
mod next;
mod timeout;

#[cfg(feature = "fs")]
pub use favicon::favicon;

pub use middleware::{Middleware, Result};
pub use next::Next;
pub use timeout::timeout;
