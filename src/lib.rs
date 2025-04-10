//! An async multi-threaded web framework for people who appreciate simplicity.
//!
//! Documentation is sparse at the moment, but the code is well-commented for
//! the most part.
//!
//! If you're interested in contributing, helping with documentation is a great
//! starting point.
//!
//! ## Hello World Example
//!
//! Below is a basic example to demonstrate how to use Via to create a simple
//! web server that responds to requests at `/hello/:name` with a personalized
//! greeting.
//! [Additional examples](https://github.com/zacharygolba/via/tree/main/examples)
//! can be found in our git repository.
//!
//! ```no_run
//! use std::process::ExitCode;
//! use via::middleware::error_boundary;
//! use via::{Next, Request, Response, Server};
//!
//! type Error = Box<dyn std::error::Error + Send + Sync>;
//!
//! async fn hello(request: Request, _: Next) -> via::Result {
//!     // Get a reference to the path parameter `name` from the request uri.
//!     let name = request.param("name").percent_decode().into_result()?;
//!
//!     // Send a plain text response with our greeting message.
//!     Response::build().text(format!("Hello, {}!", name))
//! }
//!
//! // For the sake of simplifying doctests, we're specifying that we want to
//! // use the "current_thread" runtime flavor. You'll most likely not want to
//! // specify a runtime flavor and simpy use #[tokio::main] if your deployment
//! // target has more than one CPU core.
//! #[tokio::main]
//! async fn main() -> Result<ExitCode, Error> {
//!     // Create a new application.
//!     let mut app = via::app(());
//!
//!     // Include an error boundary to catch any errors that occur downstream.
//!     app.include(error_boundary::map(|error| {
//!         eprintln!("error: {}", error);
//!         error.use_canonical_reason()
//!     }));
//!
//!     // Define a route that listens on /hello/:name.
//!     app.at("/hello/:name").respond(via::get(hello));
//!
//!     via::start(app).listen(("127.0.0.1", 8080)).await
//! }
//! ```
//!

#![allow(clippy::module_inception)]

pub mod error;
pub mod middleware;
pub mod request;
pub mod response;

mod app;
mod server;

pub use app::{app, App, Route};
pub use error::Error;
pub use middleware::method::*;
pub use middleware::{Middleware, Next, Result};
pub use request::Request;
pub use response::{Pipe, Response};
pub use server::{start, Server};

/// A type erased, dynamically dispatched [`Body`](http_body::Body).
///
pub type BoxBody = http_body_util::combinators::BoxBody<bytes::Bytes, error::DynError>;
