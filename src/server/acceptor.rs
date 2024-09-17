use std::convert::Infallible;
use std::error::Error;
use std::future::{self, Future, Ready};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
#[cfg(feature = "rustls")]
use tokio_rustls::server::TlsStream;
#[cfg(feature = "rustls")]
use tokio_rustls::{Accept, TlsAcceptor};

pub trait Acceptor: Clone {
    type Accepted: Future<Output = Result<Self::Stream, Self::Error>> + Send + Sync + 'static;
    type Stream: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static;
    type Error: Error + Send + Sync + 'static;

    fn accept(&mut self, stream: TcpStream) -> Self::Accepted;
}

#[derive(Clone, Copy)]
pub struct HttpAcceptor;

impl Acceptor for HttpAcceptor {
    type Accepted = Ready<Result<Self::Stream, Self::Error>>;
    type Stream = TcpStream;
    type Error = Infallible;

    fn accept(&mut self, stream: TcpStream) -> Self::Accepted {
        future::ready(Ok(stream))
    }
}

#[cfg(feature = "rustls")]
impl Acceptor for TlsAcceptor {
    type Accepted = Accept<TcpStream>;
    type Stream = TlsStream<TcpStream>;
    type Error = std::io::Error;

    fn accept(&mut self, stream: TcpStream) -> Self::Accepted {
        Self::accept(self, stream)
    }
}
