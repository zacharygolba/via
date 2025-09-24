use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::rustls;
use tokio_rustls::server::TlsStream;

use crate::error::{BoxError, ServerError};

pub use rustls::ServerConfig as TlsConfig;

#[derive(Clone)]
pub struct TlsAcceptor(tokio_rustls::TlsAcceptor);

impl TlsAcceptor {
    pub fn new(config: TlsConfig) -> Result<Self, BoxError> {
        Ok(Self(Arc::new(config).into()))
    }

    pub async fn accept(&self, stream: TcpStream) -> Result<TlsStream<TcpStream>, ServerError> {
        self.0.accept(stream).await.map_err(ServerError::Io)
    }
}
