use std::future::{self, Ready};
use std::io;
use tokio::net::TcpStream;

use super::Acceptor;

/// Accepts a TCP stream and returns it as-is.
///
#[derive(Clone)]
pub struct HttpAcceptor(
    // Pad HttpAcceptor with usize to avoid passing a ZST to accept.
    #[allow(dead_code)] usize,
);

impl HttpAcceptor {
    pub fn new() -> Self {
        Self(0)
    }
}

impl Acceptor for HttpAcceptor {
    type Future = Ready<Result<Self::Stream, io::Error>>;
    type Stream = TcpStream;

    fn accept(&mut self, stream: TcpStream) -> Self::Future {
        future::ready(Ok(stream))
    }
}
