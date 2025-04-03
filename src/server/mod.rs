//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod acceptor;
mod serve;
mod server;
mod stream;

pub use server::{start, Server};
