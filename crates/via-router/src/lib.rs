#![forbid(unsafe_code)]

#[cfg(feature = "lru-cache")]
mod cache;

mod path;
mod router;
mod search;

pub use path::Param;
pub use router::{Route, Router};
pub use search::{Found, Match};
