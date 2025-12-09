use std::future::Future;
use std::io;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::Arc;
use std::task::{Context, Poll, ready};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_rustls::server::{Accept, TlsAcceptor, TlsStream};

use super::super::accept;
use super::super::server::ServerConfig;
use crate::app::AppService;
use crate::error::Error;

enum Negotiate {
    Ready(TlsStream<TcpStream>),
    Pending(Accept<TcpStream>),
}

pub fn listen_rustls<State, A>(
    config: ServerConfig,
    address: A,
    tls_config: rustls::ServerConfig,
    service: AppService<State>,
) -> impl Future<Output = Result<ExitCode, Error>>
where
    A: ToSocketAddrs,
    State: Send + Sync + 'static,
{
    let handshake = {
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        Box::new(move |stream| {
            let acceptor = acceptor.clone();
            async move {
                let mut stream = Box::pin(Negotiate::Pending(acceptor.accept(stream)));
                stream.as_mut().await?;
                Ok(stream)
            }
        })
    };

    async {
        let exit = accept(
            config,
            TcpListener::bind(address).await?,
            handshake,
            service,
        );

        Ok(exit.await)
    }
}

impl AsyncRead for Negotiate {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Self::Ready(stream) = self.get_mut() {
            Pin::new(stream).poll_read(context, buf)
        } else {
            Poll::Pending
        }
    }
}

impl AsyncWrite for Negotiate {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if let Self::Ready(stream) = self.get_mut() {
            Pin::new(stream).poll_write(context, buf)
        } else {
            Poll::Pending
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        if let Self::Ready(stream) = self.get_mut() {
            Pin::new(stream).poll_flush(context)
        } else {
            Poll::Pending
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context) -> Poll<io::Result<()>> {
        if let Self::Ready(stream) = self.get_mut() {
            Pin::new(stream).poll_shutdown(context)
        } else {
            Poll::Pending
        }
    }

    fn is_write_vectored(&self) -> bool {
        true
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        context: &mut Context,
        bufs: &[io::IoSlice],
    ) -> Poll<io::Result<usize>> {
        if let Self::Ready(stream) = self.get_mut() {
            Pin::new(stream).poll_write_vectored(context, bufs)
        } else {
            Poll::Pending
        }
    }
}

impl Future for Negotiate {
    type Output = io::Result<()>;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let this = self.get_mut();

        if let Self::Pending(accept) = this {
            let stream = ready!(Pin::new(accept).poll(context)?);
            *this = Negotiate::Ready(stream);
        }

        Poll::Ready(Ok(()))
    }
}
