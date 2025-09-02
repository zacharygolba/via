#![forbid(unsafe_code)]

mod path;
mod router;

pub use path::Param;
pub use router::{Binding, Route, RouteStack, Router};
