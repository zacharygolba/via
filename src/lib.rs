#![allow(clippy::module_inception)]

pub mod body;
pub mod middleware;
pub mod request;
pub mod response;
pub mod server;

mod app;
mod error;
mod router;

pub use http;

pub use app::{new, App};
pub use error::{Error, Result};
pub use middleware::allow_method::{connect, delete, get, head, options, patch, post, put};
pub use middleware::{ErrorBoundary, Middleware, Next};
pub use request::Request;
pub use response::Response;
pub use router::Endpoint;
pub use server::Server;

use router::Router;
