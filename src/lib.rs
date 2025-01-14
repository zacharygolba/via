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
//! ```
//! use std::process::ExitCode;
//! use via::middleware::error_boundary;
//! use via::{BoxError, Error, Next, Request, Server};
//!
//! async fn hello(request: Request, _: Next) -> Result<String, Error> {
//!     // Get a reference to the path parameter `name` from the request uri.
//!     let name = request.param("name").percent_decode().into_result()?;
//!
//!     // Send a plain text response with our greeting message.
//!     Ok(format!("Hello, {}!", name))
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<ExitCode, BoxError> {
//!     // Create a new application.
//!     let mut app = via::new(());
//!
//!     // Include an error boundary to catch any errors that occur downstream.
//!     app.include(error_boundary::catch(|error, _| {
//!         eprintln!("Error: {}", error);
//!     }));
//!
//!     // Define a route that listens on /hello/:name.
//!     app.at("/hello/:name").respond(via::get(hello));
//!
//!     // Start the server.
//!     Server::new(app).listen(("127.0.0.1", 8080)).await
//! }
//! ```
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

pub use app::{new, App};
pub use error::{BoxError, Error};
pub use middleware::filter_method::{connect, delete, get, head, options, patch, post, put, trace};
pub use middleware::Next;
pub use request::Request;
pub use response::{Pipe, Response};
pub use router::Route;
pub use server::Server;
