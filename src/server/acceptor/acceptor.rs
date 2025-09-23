use std::future::{self, Future, Ready};
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

/// A trait for types that can accept a TcpStream.
///
pub trait Acceptor: Send + Sync {
    type Future: Future<Output = io::Result<Self::Stream>> + Send + Sync;
    type Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin;

    /// Defines how to accept a TcpStream. If the connection is served over TLS,
    /// this is where the TLS handshake would be performed.
    ///
    fn accept(&self, stream: TcpStream) -> Self::Future;
}

impl<T> Acceptor for T
where
    T: Fn(TcpStream) -> TcpStream + Send + Sync,
{
    type Future = Ready<Result<Self::Stream, io::Error>>;
    type Stream = TcpStream;

    fn accept(&self, stream: TcpStream) -> Self::Future {
        future::ready(Ok(self(stream)))
    }
}
