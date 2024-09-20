mod acceptor;

#[cfg(not(feature = "rustls"))]
mod http_acceptor;

#[cfg(feature = "rustls")]
mod rustls_acceptor;

pub use acceptor::Acceptor;

#[cfg(not(feature = "rustls"))]
pub use http_acceptor::HttpAcceptor;

#[cfg(feature = "rustls")]
pub use rustls_acceptor::RustlsAcceptor;
