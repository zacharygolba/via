#![forbid(unsafe_code)]

#[cfg(feature = "lru-cache")]
mod cache;

mod path;
mod tree;

pub use path::Param;
pub use tree::{Binding, Builder, MatchCond, Route, Router};
