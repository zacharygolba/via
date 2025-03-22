#![forbid(unsafe_code)]

#[cfg(feature = "lru-cache")]
mod cache;

mod error;
mod path;
mod router;

pub use error::Error;
pub use path::Param;
pub use router::{Route, Router};
