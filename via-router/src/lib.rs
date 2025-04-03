#![forbid(unsafe_code)]

pub mod binding;

mod path;
mod router;

pub use binding::MatchKind;
pub use path::Param;
pub use router::{Node, Route, Router};
