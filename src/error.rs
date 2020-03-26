use crate::Response;

pub use ::error::*;
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;
