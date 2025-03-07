use std::error::Error;
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A trait for types that can accept a TcpStream.
///
pub trait Acceptor: Send + Sync {
    type Accepted: Future<Output = Result<Self::Stream, Self::Error>> + Send + Sync;
    type Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin;
    type Error: Error + Send + Sync;

    /// Defines how to accept a TcpStream. If the connection is served over TLS,
    /// this is where the TLS handshake would be performed.
    ///
    fn accept(&mut self, stream: TcpStream) -> Self::Accepted;
}
