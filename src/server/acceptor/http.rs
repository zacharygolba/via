use futures_core::future::BoxFuture;
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
    type Stream = TcpStream;

    fn accept(&self, stream: TcpStream) -> BoxFuture<'static, io::Result<Self::Stream>> {
        Box::pin(async { Ok(stream) })
    }
}
