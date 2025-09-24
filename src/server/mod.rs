//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod io;
mod server;
mod tls;

pub use server::{Server, serve};
