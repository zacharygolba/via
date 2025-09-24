#[cfg(feature = "native-tls")]
mod native;

#[cfg(feature = "rustls")]
mod rustls;

#[cfg(feature = "native-tls")]
pub use native::{TlsAcceptor, TlsConfig};

#[cfg(feature = "rustls")]
pub use rustls::{TlsAcceptor, TlsConfig};
