#![forbid(unsafe_code)]

#[cfg(feature = "lru-cache")]
mod cache;

mod path;
mod router;

pub use path::Param;
pub use router::{Match, Node, Route, Router};
