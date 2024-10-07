//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod acceptor;
mod serve;
mod server;
mod service;
mod shutdown;

pub use server::Server;
