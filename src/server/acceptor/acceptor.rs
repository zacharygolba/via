use std::future::Future;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A trait for types that can accept a TcpStream.
///
pub trait Acceptor: Clone + Send + Sync {
    type Future: Future<Output = Result<Self::Stream, io::Error>> + Send + Sync;
    type Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin;

    /// Defines how to accept a TcpStream. If the connection is served over TLS,
    /// this is where the TLS handshake would be performed.
    ///
    fn accept(&mut self, stream: TcpStream) -> Self::Future;
}
