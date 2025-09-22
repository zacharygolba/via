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
//! use via::builtin::rescue;
//! use via::{BoxError, Next, Request, Response, Server};
//!
//! async fn hello(request: Request, _: Next) -> via::Result {
//!     // Get a reference to the path parameter `name` from the request uri.
//!     let name = request.param("name").percent_decode().into_result()?;
//!
//!     // Send a plain text response with our greeting message.
//!     Response::build().text(format!("Hello, {}!", name))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<ExitCode, BoxError> {
//!     // Create a new application.
//!     let mut app = via::app(());
//!
//!     // Capture errors from downstream, log them, and map them into responses.
//!     // Upstream middleware remains unaffected and continues execution.
//!     app.include(rescue::inspect(|error| eprintln!("error: {}", error)));
//!
//!     // Define a route that listens on /hello/:name.
//!     app.at("/hello/:name").respond(via::get(hello));
//!
//!     via::start(app).listen(("127.0.0.1", 8080)).await
//! }
//! ```
//!

#![allow(clippy::module_inception)]

pub mod builtin;
pub mod request;
pub mod response;

mod app;
mod error;
mod middleware;
mod next;
mod server;

pub use app::{App, Route, app};
pub use builtin::allow::{delete, get, head, options, patch, post, put, trace};
pub use error::{BoxError, Error, ErrorMessage};
pub use middleware::{BoxFuture, Middleware, Result};
pub use next::Next;
pub use request::Request;
pub use response::{Pipe, Response};
pub use server::{Server, start};

#[cfg(feature = "ws")]
pub use builtin::ws::ws;
