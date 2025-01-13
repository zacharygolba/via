//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod acceptor;
mod serve;
mod server;
mod shutdown;

pub use server::Server;
