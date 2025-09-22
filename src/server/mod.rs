//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod acceptor;
mod io;
mod server;

pub use server::{Server, serve};
