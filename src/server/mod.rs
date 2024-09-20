mod acceptor;
mod io_stream;
mod serve;
mod server;
mod service;
mod shutdown;

pub use io_stream::IoStream;
pub use server::Server;

#[cfg(feature = "rustls")]
use acceptor::RustlsAcceptor;

#[cfg(not(feature = "rustls"))]
use acceptor::HttpAcceptor;
