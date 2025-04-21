use futures_core::future::BoxFuture;
use std::future::{self, Ready};
use std::io;
use tokio::net::TcpStream;

use super::Acceptor;

/// Accepts a TCP stream and returns it as-is.
///
#[derive(Clone, Copy)]
pub struct HttpAcceptor;

impl Acceptor for HttpAcceptor {
    type Stream = TcpStream;
    type Future = Ready<io::Result<Self::Stream>>;

    #[inline]
    fn accept(&self, stream: TcpStream) -> Self::Future {
        future::ready(Ok(stream))
    }
}
