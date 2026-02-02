use std::error::Error;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

#[cfg(feature = "native-tls")]
pub use native::NativeTlsAcceptor;

#[cfg(feature = "rustls")]
pub use rustls::RustlsAcceptor;

pub struct TcpAcceptor;

pub trait Acceptor {
    type Io: AsyncRead + AsyncWrite;
    type Error: Error;

    #[cfg_attr(not(any(feature = "native-tls", feature = "rustls")), allow(dead_code))]
    fn accept(
        &self,
        io: TcpStream,
    ) -> impl Future<Output = Result<Self::Io, Self::Error>> + Send + 'static;
}

impl Acceptor for TcpAcceptor {
    type Io = TcpStream;
    type Error = io::Error;

    #[allow(clippy::manual_async_fn)]
    fn accept(
        &self,
        _: TcpStream,
    ) -> impl Future<Output = Result<Self::Io, Self::Error>> + Send + 'static {
        async { unreachable!() }
    }
}

#[cfg(feature = "native-tls")]
mod native {
    use native_tls::{Identity, Protocol};
    use std::sync::Arc;
    use tokio::net::TcpStream;
    use tokio_native_tls::{TlsAcceptor, TlsStream};

    use super::Acceptor;

    pub struct NativeTlsAcceptor(Arc<TlsAcceptor>);

    impl NativeTlsAcceptor {
        pub fn new(identity: Identity) -> Self {
            Self(Arc::new(TlsAcceptor::from(
                native_tls::TlsAcceptor::builder(identity)
                    .min_protocol_version(Some(Protocol::Tlsv12))
                    .build()
                    .expect("tls config is invalid or missing"),
            )))
        }
    }

    impl Acceptor for NativeTlsAcceptor {
        type Io = TlsStream<TcpStream>;
        type Error = native_tls::Error;

        fn accept(
            &self,
            io: TcpStream,
        ) -> impl Future<Output = Result<Self::Io, Self::Error>> + Send + 'static {
            let acceptor = Arc::clone(&self.0);
            async move { acceptor.accept(io).await }
        }
    }
}

#[cfg(feature = "rustls")]
mod rustls {
    use rustls::ServerConfig;
    use std::{io, sync::Arc};
    use tokio::net::TcpStream;
    use tokio_rustls::server::{TlsAcceptor, TlsStream};

    use super::Acceptor;

    pub struct RustlsAcceptor(TlsAcceptor);

    impl RustlsAcceptor {
        pub fn new(rustls_config: ServerConfig) -> Self {
            Self(TlsAcceptor::from(Arc::new(rustls_config)))
        }
    }

    impl Acceptor for RustlsAcceptor {
        type Io = TlsStream<TcpStream>;
        type Error = io::Error;

        fn accept(
            &self,
            io: TcpStream,
        ) -> impl Future<Output = Result<Self::Io, Self::Error>> + Send + 'static {
            self.0.accept(io)
        }
    }
}
