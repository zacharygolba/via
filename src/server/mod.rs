//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod acceptor;
mod error;
mod server;

pub use server::{Server, start};
