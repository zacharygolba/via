use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{Accept, TlsAcceptor};

use super::Acceptor;

#[derive(Clone)]
pub struct RustlsAcceptor {
    acceptor: TlsAcceptor,
}

impl RustlsAcceptor {
    pub fn new(config: Arc<rustls::ServerConfig>) -> Self {
        Self {
            acceptor: config.into(),
        }
    }
}

impl Acceptor for RustlsAcceptor {
    type Accepted = Accept<TcpStream>;
    type Stream = TlsStream<TcpStream>;
    type Error = std::io::Error;

    fn accept(&mut self, stream: TcpStream) -> Self::Accepted {
        self.acceptor.accept(stream)
    }
}
