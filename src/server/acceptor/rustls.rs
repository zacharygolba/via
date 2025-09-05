use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{Accept, rustls};

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
    type Future = Accept<TcpStream>;
    type Stream = TlsStream<TcpStream>;

    fn accept(&mut self, stream: TcpStream) -> Self::Future {
        self.acceptor.accept(stream)
    }
}
