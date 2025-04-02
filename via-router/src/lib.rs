#![allow(clippy::while_let_on_iterator)]
#![forbid(unsafe_code)]

pub mod binding;

mod path;
mod router;

pub use binding::{Binding, MatchCond, MatchKind};
pub use path::Param;
pub use router::{Node, Route, Router};
