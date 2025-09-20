use std::future::{self, Ready};
use std::io;
use tokio::net::TcpStream;

use super::Acceptor;

/// Accepts a TCP stream and returns it as-is.
///
#[derive(Clone)]
pub struct HttpAcceptor;

impl HttpAcceptor {
    pub fn new() -> Self {
        Self
    }
}

impl Acceptor for HttpAcceptor {
    type Future = Ready<Result<Self::Stream, io::Error>>;
    type Stream = TcpStream;

    fn accept(&mut self, stream: TcpStream) -> Self::Future {
        future::ready(Ok(stream))
    }
}
