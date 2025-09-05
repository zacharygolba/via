//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod acceptor;
mod server;
mod stream;

pub use server::{Server, start};
