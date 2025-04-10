#![allow(unused_imports)]

mod acceptor;

#[cfg(feature = "rustls")]
mod rustls;

#[cfg(not(feature = "rustls"))]
mod http;

pub use acceptor::Acceptor;

#[cfg(feature = "rustls")]
pub use rustls::{RustlsAcceptor, RustlsConfig};

#[cfg(not(feature = "rustls"))]
pub use http::HttpAcceptor;
