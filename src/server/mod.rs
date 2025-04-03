//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod acceptor;
mod error;
mod serve;
mod server;
mod stream;

pub use server::{start, Server};
