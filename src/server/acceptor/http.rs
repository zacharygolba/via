use std::convert::Infallible;
use std::future::{self, Ready};
use tokio::net::TcpStream;

use super::Acceptor;

/// Accepts a TCP stream and returns it as-is.
///
pub struct HttpAcceptor;

impl Acceptor for HttpAcceptor {
    type Accepted = Ready<Result<Self::Stream, Self::Error>>;
    type Stream = TcpStream;
    type Error = Infallible;

    fn accept(&mut self, stream: TcpStream) -> Self::Accepted {
        future::ready(Ok(stream))
    }
}
