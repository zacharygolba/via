use std::future::Future;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A trait for types that can accept a TcpStream.
///
pub trait Acceptor {
    type Stream: AsyncRead + AsyncWrite + Send + Unpin;
    type Future: Future<Output = io::Result<Self::Stream>> + Send;

    /// Defines how to accept a TcpStream. If the connection is served over TLS,
    /// this is where the TLS handshake would be performed.
    ///
    fn accept(&self, stream: TcpStream) -> Self::Future;
}
