//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod acceptor;
mod server;
mod stream;
mod util;

pub use server::{start, Server};
