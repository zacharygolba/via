use futures_core::future::BoxFuture;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::{rustls, Accept};

pub use rustls::ServerConfig as RustlsConfig;

use super::Acceptor;

#[derive(Clone)]
pub struct RustlsAcceptor {
    acceptor: TlsAcceptor,
}

impl RustlsAcceptor {
    pub fn new(config: Arc<RustlsConfig>) -> Self {
        Self {
            acceptor: config.into(),
        }
    }
}

impl Acceptor for RustlsAcceptor {
    type Stream = TlsStream<TcpStream>;
    type Future = Accept<TcpStream>;

    #[inline]
    fn accept(&self, stream: TcpStream) -> Self::Future {
        self.acceptor.accept(stream)
    }
}
