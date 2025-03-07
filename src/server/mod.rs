//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod acceptor;
mod io_stream;
mod serve;
mod server;

pub use server::{start, Server};
