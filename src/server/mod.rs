//! Serve an [App](crate::App) over HTTP or HTTPS.
//!

mod accept;
mod io;
mod server;

#[cfg(any(feature = "native-tls", feature = "rustls"))]
mod tls;

pub use server::Server;

use accept::accept;
use server::ServerConfig;
