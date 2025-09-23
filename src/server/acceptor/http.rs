use std::future::{self, Ready};
use std::io;
use tokio::net::TcpStream;

use super::Acceptor;

/// Accepts a TCP stream and returns it as-is.
///
pub struct HttpAcceptor;

impl HttpAcceptor {
    pub fn new() -> Self {
        Self
    }
}

impl Acceptor for HttpAcceptor {
    type Future = Ready<Result<Self::Stream, io::Error>>;
    type Stream = TcpStream;

    #[inline]
    fn accept(&self, stream: TcpStream) -> Self::Future {
        future::ready(Ok(stream))
    }
}
