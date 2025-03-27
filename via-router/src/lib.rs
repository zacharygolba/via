#![forbid(unsafe_code)]

#[cfg(feature = "lru-cache")]
mod cache;

mod error;
mod path;
mod tree;

pub use error::Error;
pub use path::{Param, Pattern};
pub use tree::{Binding, MatchCond, Route, Router};
