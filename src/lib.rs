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
//! use via::{App, Error, Next, Request, Response, Server};
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
//! async fn main() -> Result<ExitCode, Error> {
//!     let mut app = App::new(());
//!
//!     // Define a route that listens on /hello/:name.
//!     app.route("/hello/:name").respond(via::get(hello));
//!
//!     Server::new(app).listen(("127.0.0.1", 8080)).await
//! }
//! ```
//!

#![allow(clippy::module_inception)]

pub mod error;
pub mod request;
pub mod response;

#[cfg(feature = "ws")]
pub mod ws;

mod allow;
mod app;
mod cookies;
mod middleware;
mod next;
mod payload;
mod server;
mod timeout;
mod util;

pub use allow::{Allow, connect, delete, get, head, options, patch, post, put, trace};
pub use app::{App, Route};
pub use cookies::Cookies;
pub use error::Error;
pub use middleware::{BoxFuture, Middleware, Result};
pub use next::Next;
pub use payload::Payload;
pub use request::Request;
pub use response::Response;
pub use server::Server;
pub use timeout::Timeout;

#[cfg(feature = "ws")]
#[doc(inline)]
pub use ws::ws;
