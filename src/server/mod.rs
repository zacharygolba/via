//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod acceptor;
mod conn;
mod server;

pub use server::{start, Server};
