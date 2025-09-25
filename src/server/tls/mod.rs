#[cfg(feature = "native-tls")]
mod native;

#[cfg(feature = "rustls")]
mod rustls;

#[cfg(feature = "native-tls")]
pub use native::listen;

#[cfg(feature = "rustls")]
pub use rustls::listen;
