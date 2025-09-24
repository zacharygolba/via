use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;
use tokio_native_tls::native_tls::Protocol;

use crate::error::{BoxError, ServerError};

pub use native_tls::Identity as TlsConfig;

#[cfg(all(feature = "http1", not(feature = "http2")))]
const MIN_PROTOCOL_VERSION: Protocol = Protocol::Tlsv10;

#[cfg(feature = "http2")]
const MIN_PROTOCOL_VERSION: Protocol = Protocol::Tlsv12;

#[derive(Clone)]
pub struct TlsAcceptor(tokio_native_tls::TlsAcceptor);

impl TlsAcceptor {
    pub fn new(config: TlsConfig) -> Result<Self, BoxError> {
        let acceptor = native_tls::TlsAcceptor::builder(config)
            .min_protocol_version(Some(MIN_PROTOCOL_VERSION))
            .build()?;

        Ok(Self(acceptor.into()))
    }

    pub async fn accept(&self, stream: TcpStream) -> Result<TlsStream<TcpStream>, ServerError> {
        let result = self.0.accept(stream).await;
        result.map_err(|error| ServerError::Handshake(error.into()))
    }
}
