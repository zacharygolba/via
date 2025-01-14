//! An async multi-threaded web framework for people who appreciate simplicity.
//!
//! Documentation is sparse at the moment, but the code is well-commented for
//! the most part.
//!
//! If you're interested in contributing, helping with documentation is a great
//! starting point.
//!
//! Check out the
//! [official examples](https://github.com/zacharygolba/via/tree/main/examples)
//! to see how to use Via.
//!

#![allow(clippy::module_inception)]

pub mod body;
pub mod middleware;
pub mod request;
pub mod response;

mod app;
mod error;
mod router;
mod server;

pub use http;

pub use app::{new, App};
pub use error::{BoxError, Error};
pub use middleware::allow_method::{connect, delete, get, head, options, patch, post, put};
pub use middleware::{Middleware, Next, Result};
pub use request::Request;
pub use response::{Pipe, Response};
pub use router::Endpoint;
pub use server::Server;
