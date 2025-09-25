//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod io;
mod server;

#[cfg(any(feature = "native-tls", feature = "rustls"))]
mod tls;

use accept::accept;
pub use server::{Server, serve};
